# Operations

## Upgrades

1. Read the release notes. Before 1.0, the API (`/api/v0`) and behaviour may change between minor versions.
2. Back up the database (see below).
3. Deploy the new image. Migrations from `backend/migrations` run at startup, before the server listens. With several replicas, the first one to start applies them. The others wait on sqlx's migration lock.

You cannot downgrade: migrations only go forward. To go back, restore the backup taken before the upgrade.

Installation-specific steps are in [Docker](install/docker.md#upgrade), [Compose](install/compose.md#upgrade) and [Kubernetes](install/kubernetes.md).

## Backups

All state lives in PostgreSQL. That covers users, sessions, settings, subscriptions, groups, playlists, likes, history, blocked channels and hidden videos, watch parties and the video metadata cache.

`/var/lib/oxistream` only holds RustyPipe's cache and does not need a backup.

```sh
pg_dump -Fc -d "$OXISTREAM_DATABASE_URL" > oxistream-$(date +%F).dump
pg_restore --clean --if-exists -d "$OXISTREAM_DATABASE_URL" oxistream-YYYY-MM-DD.dump
```

Restore into the same or a newer PostgreSQL major version, with the server stopped. For Compose, see [Compose backups](install/compose.md#backups). On CloudNativePG, use its backup support (Barman or volume snapshots).

## Multiple replicas

Any replica can serve any request. They coordinate through PostgreSQL.

| Requirement | Why |
| --- | --- |
| Same `OXISTREAM_PROXY_SECRET` everywhere | Proxy URLs and the OIDC login cookie are HMAC-signed. A URL signed by one replica must verify on another. |
| One egress IP | YouTube binds stream URLs to the IP that requested them. Each replica resolves streams with its own player cache, so a replica only uses URLs it fetched itself. Replicas behind different IPs still look like several clients to YouTube. Different IPs also make blocks harder to diagnose. |
| NTP-synced clocks | Watch party positions (`position_at`) and pongs use each replica's wall clock. Clock skew between replicas adds to party drift. |
| Direct or session-mode database connections | The watch party listener uses `LISTEN`. Transaction-mode poolers (PgBouncer's default mode) break it. |
| Connection budget | Each replica opens up to `OXISTREAM_DATABASE_MAX_CONNECTIONS` (default 10) connections. |

What runs where:

| Task | Interval | Coordination |
| --- | --- | --- |
| Feed worker: channels and followed YouTube playlists | 1 min tick, each item every `OXISTREAM_FEED_INTERVAL_MINUTES` | Advisory lock: one replica per run. |
| History retention cleanup | hourly | Advisory lock. |
| Expired sessions and used back-channel logout `jti`s | hourly | Idempotent deletes, on every replica. |
| Watch party cleanup: stale participants (2 min without a heartbeat), rooms idle for 24 h | every 5 min | Any replica. |
| Watch party sync | on change | `LISTEN`/`NOTIFY` on `watch_rooms`, `watch_room_status`, `watch_room_chat`, `watch_room_reaction`. |

These caches are in memory, per replica:

- player responses
- trending pages (30 min)
- search suggestions (10 min)
- SponsorBlock segments (1 h; 15 min when empty; 1 min after errors)

Hit rates therefore drop with more replicas. Sticky sessions are not required.

## Observability

### Logs

Logs go to stdout. `OXISTREAM_LOG_FORMAT=json` gives one JSON object per line. `OXISTREAM_LOG` takes a `tracing` filter, e.g. `info,oxistream_server=debug`.

Each HTTP request is logged with method, URI, version and status. Client IPs are not logged, and Atom feed tokens in paths appear as `<redacted>`. At `debug`, the parsed configuration is logged with secrets redacted.

Startup warnings worth acting on:

| Message | Meaning |
| --- | --- |
| `rustypipe-botguard not found, PO tokens are unavailable` | See [botguard](#botguard). |
| `session cookies are sent without the Secure flag` | `OXISTREAM_COOKIE_SECURE=false`. |
| `rate limiting is disabled` | `OXISTREAM_RATE_LIMIT=false`. |

### Health

| Endpoint | 200 when | Use |
| --- | --- | --- |
| `/healthz` | the process serves HTTP | liveness, startup |
| `/readyz` | the database answers within 2 s and, with OIDC, discovery has succeeded | readiness |

Neither endpoint needs authentication or counts toward rate limiting. `/readyz` answers 503 with `database unavailable` or `OIDC provider not discovered`.

### Metrics

`/metrics` serves Prometheus text format. It has no authentication, so restrict it at the reverse proxy or network level if needed.

| Metric | Type | Labels |
| --- | --- | --- |
| `http_requests_total` | counter | `method`, `path` (route pattern), `status` |
| `http_request_duration_seconds` | histogram | `method`, `path`, `status` |
| `proxy_upstream_responses_total` | counter | `kind` (stream, image, subtitle), `status` |
| `proxy_bytes_total` | counter | `kind` |
| `feed_channel_fetches_total` | counter | `result`: `ok`, `rate_limited`, `error` |
| `feed_playlist_fetches_total` | counter | `result`: `ok`, `rate_limited`, `error` |

The admin [status page](administration.md#status) adds feed worker and database details.

## Rate limiting

On by default. `/api` and `/proxy` share a per-client-IP token bucket: 50 requests per second sustained, burst 200 (`OXISTREAM_RATE_LIMIT_PER_SECOND`, `OXISTREAM_RATE_LIMIT_BURST`). IPv6 clients are grouped by /64. Over the limit, the server answers 429 with `Retry-After`. Login and registration have [stricter limits](authentication.md#local-accounts).

A video playback makes many `/proxy` range requests. If clients see 429s during playback, raise the burst. Behind a reverse proxy, set `OXISTREAM_TRUSTED_PROXY_HEADER`, or all users share one bucket. See [Reverse proxy](reverse-proxy.md#client-ip).

Watch party WebSockets have their own per-connection limits.

## Troubleshooting

### YouTube blocks and 403s

Oxistream talks to YouTube from the server's IP and never with an account. Datacenter IPs are blocked more often than residential ones.

| Symptom | Likely cause | What to do |
| --- | --- | --- |
| Video shows "rate limited", search or channel pages fail | YouTube is throttling the IP | Wait. Reduce load (fewer replicas, a longer `OXISTREAM_FEED_INTERVAL_MINUTES`). Use another egress IP. |
| Feed runs stop early, `feed_channel_fetches_total{result="rate_limited"}` rises | Same | Same. The worker stops a run when YouTube blocks, and resumes on the next tick. |
| Playback fails with upstream 403 | Stream URL expired, or used from another IP | On a 403 the proxy refetches player data once. Make sure all replicas share one egress IP. |
| Playback starts after a delay of several seconds | The stream only becomes valid after a pre-roll ad would have ended | Expected for some player clients. The proxy waits for it. |
| Many videos fail to play | Missing PO tokens | Check [botguard](#botguard). |

YouTube-side fixes (deobfuscation, clients, parsing) land in Oxistream's RustyPipe fork. When YouTube changes break playback, update to a newer release.

### Botguard

`rustypipe-botguard` generates the PO tokens YouTube requires for many streams. The image bundles it at `/usr/local/bin/rustypipe-botguard`. Outside the image, install it and set `OXISTREAM_BOTGUARD_BIN`, or put it on `PATH`.

The startup log and the admin status page show whether it was found, and its version. Its snapshot is kept in `OXISTREAM_YOUTUBE_CACHE_DIR`. On a read-only root filesystem, this directory and `/tmp` must be writable.

### `/readyz` fails

| Body | Cause |
| --- | --- |
| `database unavailable` | The database is unreachable or slower than 2 s. Check `OXISTREAM_DATABASE_URL`, the network, and the pool size against `max_connections`. |
| `OIDC provider not discovered` | The server cannot fetch `{issuer}/.well-known/openid-configuration`. Check `OXISTREAM_OIDC_ISSUER` (exact issuer, no trailing path differences), DNS and TLS from the container. The server retries with backoff, up to once a minute, and logs each failure. |

### Sign-in does not stick

- Over plain HTTP on a host other than `localhost`, the browser drops the `Secure` cookie. Use HTTPS, or `OXISTREAM_COOKIE_SECURE=false` for testing.
- 403 on sign-in or any form: the Origin check failed. Set `OXISTREAM_PUBLIC_URL`, or pass the original `Host` header. See [Reverse proxy](reverse-proxy.md#origin-check).

### Proxy links return 403 "invalid signature" or "link expired"

The replicas have different `OXISTREAM_PROXY_SECRET` values, the secret changed, or a page stayed open longer than `OXISTREAM_PROXY_STREAM_TTL_SECS`. Reloading the page fetches fresh links.
