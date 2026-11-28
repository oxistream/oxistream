# Troubleshooting

Start with the admin [status page](administration/index.md#status) and the server log. Both show the botguard version, the database, the feed worker and SSO discovery.

## Playback and YouTube

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

### 429 Too Many Requests during playback

A video makes many `/proxy` range requests. Behind a reverse proxy without `OXISTREAM_TRUSTED_PROXY_HEADER`, all users share one rate-limit bucket. Set the header, and raise `OXISTREAM_RATE_LIMIT_BURST` if single clients still hit the limit. See [rate limiting](advanced/scaling.md#rate-limiting).

### Proxy links return 403 "invalid signature" or "link expired"

The replicas have different `OXISTREAM_PROXY_SECRET` values, the secret changed, or a page stayed open longer than `OXISTREAM_PROXY_STREAM_TTL_SECS`. Reloading the page fetches fresh links.

## Sign-in

### Sign-in does not stick

- Over plain HTTP on a host other than `localhost`, the browser drops the `Secure` cookie. Use HTTPS, or `OXISTREAM_COOKIE_SECURE=false` for testing.
- 403 on sign-in or any form: the Origin check failed. Set `OXISTREAM_PUBLIC_URL`, or pass the original `Host` header. See [Reverse proxy](advanced/reverse-proxy.md#origin-check).

### SSO sign-in fails

- "The sign-in provider cannot be reached right now": OIDC discovery has not succeeded yet. `/readyz` answers `OIDC provider not discovered` meanwhile, see [below](#readyz-fails).
- No SSO button: OIDC needs both `OXISTREAM_OIDC_ISSUER` and `OXISTREAM_OIDC_CLIENT_ID`.
- The provider rejects the redirect URI: register exactly `{PUBLIC_URL}/api/v0/auth/oidc/callback`, with `OXISTREAM_PUBLIC_URL` as the external origin. See [OIDC setup](advanced/authentication.md#setup).
- A user is not admin although they are in the admin group: the value must match the groups claim exactly, e.g. `/admins` for Keycloak's full group paths. See [admin role from groups](advanced/authentication.md#admin-role-from-groups).
- An admin lost the role after signing in: with an admin group, every sign-in syncs the role from the groups claim, even for the last admin. Add them to the group at the provider. Local registrations do not become admin while an admin group is set.
- No admin although the first user signed in through SSO: SSO accounts become admin only through `OXISTREAM_OIDC_ADMIN_GROUP`. Set it, or, without an admin group, register a local account: while there is no active admin, it becomes one. See [first admin](advanced/authentication.md#first-admin).

## Reverse proxy

### Watch parties do not connect or keep disconnecting

The proxy does not forward the WebSocket upgrade for `/api/v0/rooms/{id}/ws`, or closes idle connections early. Forward `Upgrade` and `Connection`, and allow long reads. See the [reverse proxy examples](advanced/reverse-proxy.md#examples).

### Subscription import fails with 413

The proxy limits the request body below the import's 5 MB. With nginx, raise `client_max_body_size`.

## Health checks

### `/readyz` fails

| Body | Cause |
| --- | --- |
| `database unavailable` | The database is unreachable or slower than 2 s. Check `OXISTREAM_DATABASE_URL`, the network, and the pool size against `max_connections`. |
| `OIDC provider not discovered` | The server cannot fetch `{issuer}/.well-known/openid-configuration`. Check `OXISTREAM_OIDC_ISSUER` (exact issuer, no trailing path differences), DNS and TLS from the container. The server retries with backoff, up to once a minute, and logs each failure. |

### The server does not start

A required variable is missing or a value is invalid. The error names the variable. See [Configuration](configuration.md).

`OXISTREAM_OIDC_ADMIN_GROUP is required when OIDC is enabled and OXISTREAM_AUTH_LOCAL_ENABLED is false`: SSO-only instances get their admins from the identity provider. Set the admin group, see [admin role from groups](advanced/authentication.md#admin-role-from-groups).
