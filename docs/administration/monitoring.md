# Monitoring

## Logs

Logs go to stdout. `OXISTREAM_LOG_FORMAT=json` gives one JSON object per line. `OXISTREAM_LOG` takes a `tracing` filter, e.g. `info,oxistream_server=debug`.

Each HTTP request is logged with method, URI, version and status. Client IPs are not logged, and Atom feed tokens in paths appear as `<redacted>`. At `debug`, the parsed configuration is logged with secrets redacted.

Startup warnings worth acting on:

| Message | Meaning |
| --- | --- |
| `rustypipe-botguard not found, PO tokens are unavailable` | See [botguard](../troubleshooting.md#botguard). |
| `session cookies are sent without the Secure flag` | `OXISTREAM_COOKIE_SECURE=false`. |
| `rate limiting is disabled` | `OXISTREAM_RATE_LIMIT=false`. |

## Health

| Endpoint | 200 when | Use |
| --- | --- | --- |
| `/healthz` | the process serves HTTP | liveness, startup |
| `/readyz` | the database answers within 2 s and, with OIDC, discovery has succeeded | readiness |

Neither endpoint needs authentication or counts toward rate limiting. `/readyz` answers 503 with `database unavailable` or `OIDC provider not discovered`.

## Metrics

`/metrics` serves Prometheus text format. It has no authentication, so restrict it at the reverse proxy or network level if needed.

| Metric | Type | Labels |
| --- | --- | --- |
| `http_requests_total` | counter | `method`, `path` (route pattern), `status` |
| `http_request_duration_seconds` | histogram | `method`, `path`, `status` |
| `proxy_upstream_responses_total` | counter | `kind` (stream, image, subtitle), `status` |
| `proxy_bytes_total` | counter | `kind` |
| `feed_channel_fetches_total` | counter | `result`: `ok`, `rate_limited`, `error` |
| `feed_playlist_fetches_total` | counter | `result`: `ok`, `rate_limited`, `error` |

The admin [status page](index.md#status) adds feed worker and database details.

