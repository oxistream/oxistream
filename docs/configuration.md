---
title: Configuration
nav_order: 4
---

# Configuration

Oxistream is configured only through `OXISTREAM_*` environment variables. There are no config files. An empty value counts as unset. If a required variable is missing or a value is invalid, the server does not start, and the error names the variable.

Booleans accept `true`/`false`, `1`/`0`, `yes`/`no` and `on`/`off`, in any case.

## Core

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_DATABASE_URL` | – | yes | PostgreSQL connection URL, e.g. `postgres://oxistream:secret@db:5432/oxistream`. Needs PostgreSQL 18 or newer. Migrations run at startup. |
| `OXISTREAM_PROXY_SECRET` | – | yes | HMAC key for signed proxy URLs and the OIDC login cookie. At least 32 characters (`openssl rand -base64 48`). Must be the same on all replicas. Changing it invalidates existing proxy links and pending SSO logins. |
| `OXISTREAM_BIND` | `0.0.0.0:8080` | no | Listen address (`host:port`). The image sets it to the default. |
| `OXISTREAM_DATABASE_MAX_CONNECTIONS` | `10` | no | Connection pool size per replica. |
| `OXISTREAM_PUBLIC_URL` | – | with OIDC | External origin, e.g. `https://oxistream.example.com`. Only scheme, host and port are kept. Required when OIDC is enabled. The [Origin check](reverse-proxy.md#origin-check) also accepts this origin, and Atom feed links use it (otherwise they use the request's `Host`). |

## Authentication

See [Authentication](authentication.md).

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_AUTH_LOCAL_ENABLED` | `true` | no | Username and password sign-in. |
| `OXISTREAM_REGISTRATION_ENABLED` | `true` | no | Self-registration of local accounts. Does not affect OIDC provisioning. |
| `OXISTREAM_REQUIRE_LOGIN` | `false` | no | Rejects anonymous API and proxy requests with 401. Guests can still join a watch party by its invite link and use the app while they are in it. |
| `OXISTREAM_SESSION_TTL_DAYS` | `30` | no | Session lifetime in days since last use (sliding). At least 1. |
| `OXISTREAM_COOKIE_SECURE` | `true` | no | `Secure` flag on cookies. Set it to `false` only for plain-HTTP testing on a host other than `localhost`. |

### OIDC

OIDC is enabled when both `OXISTREAM_OIDC_ISSUER` and `OXISTREAM_OIDC_CLIENT_ID` are set. Setting only one of the two is an error. With OIDC enabled, `OXISTREAM_PUBLIC_URL` is required.

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_OIDC_ISSUER` | – | for OIDC | Issuer URL (`http` or `https`), e.g. `https://id.example.com/realms/main`. |
| `OXISTREAM_OIDC_CLIENT_ID` | – | for OIDC | Client ID. |
| `OXISTREAM_OIDC_CLIENT_SECRET` | – | no | Client secret. Omit it for public clients. |
| `OXISTREAM_OIDC_SCOPES` | `openid profile email` | no | Space-separated scopes. `openid` is always added. |
| `OXISTREAM_OIDC_NAME` | `SSO` | no | Label of the sign-in button ("Sign in with …"). |
| `OXISTREAM_OIDC_ADMIN_GROUP` | – | no | Members of this group are admins. Must match a claim value exactly. When unset, roles are not synced. |
| `OXISTREAM_OIDC_GROUPS_CLAIM` | `groups` | no | Claim that holds the groups. Dotted paths such as `realm_access.roles` work. |

## YouTube

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_BOTGUARD_BIN` | – | no | Path to `rustypipe-botguard`, which produces PO tokens. When unset, the server looks for the binary in the working directory and `PATH`. The image sets `/usr/local/bin/rustypipe-botguard`. |
| `OXISTREAM_YOUTUBE_CACHE_DIR` | – | no | Directory for RustyPipe's client cache and the botguard snapshot. When unset, the cache stays in memory and the snapshot goes to the temp directory. The image sets `/var/lib/oxistream`. |
| `OXISTREAM_TRENDING_REGION` | `US` | no | Two-letter country code for the trending rows. Must be a country YouTube supports; checked at startup. |
| `OXISTREAM_FEED_INTERVAL_MINUTES` | `30` | no | How often each subscribed channel and followed YouTube playlist is fetched. At least 1. |

## Stream proxy and rate limiting

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_PROXY_STREAM_TTL_SECS` | `21600` (6 h) | no | Lifetime of signed stream and subtitle URLs. |
| `OXISTREAM_PROXY_IMAGE_TTL_SECS` | `604800` (7 d) | no | Lifetime of signed image URLs. Expiry is rounded up to the day, so the URLs stay stable and browsers can cache them. |
| `OXISTREAM_RATE_LIMIT` | `true` | no | Rate limiting per client IP on `/api` and `/proxy`, plus stricter limits on login and registration. |
| `OXISTREAM_RATE_LIMIT_PER_SECOND` | `50` | no | Sustained requests per second per client. At least 1. |
| `OXISTREAM_RATE_LIMIT_BURST` | `200` | no | Burst size per client. At least 1. |
| `OXISTREAM_TRUSTED_PROXY_HEADER` | – | no | Header a trusted reverse proxy sets to the client IP, e.g. `X-Forwarded-For`. The last value in the header is used. Set it only when every request comes through that proxy. Read only while rate limiting is enabled. |

## SponsorBlock

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_SPONSORBLOCK_ENABLED` | `true` | no | Segment lookups and the SponsorBlock settings in the UI. |
| `OXISTREAM_SPONSORBLOCK_API_URL` | `https://sponsor.ajay.app` | no | SponsorBlock API server (`http` or `https`), e.g. a mirror you host yourself. |

## History

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_HISTORY_MAX_RETENTION` | `forever` | no | Longest watch history retention users may pick: `1d`, `7d`, `14d`, `30d`, `90d`, `180d`, `365d` or `forever`. A user's longer setting is capped to this value. Guests' local progress is pruned to it as well. |

## Logging

| Variable | Default | Required | Description |
| --- | --- | --- | --- |
| `OXISTREAM_LOG` | `info` | no | [`tracing` filter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html), e.g. `info,oxistream_server=debug`. |
| `OXISTREAM_LOG_FORMAT` | `pretty` | no | `pretty` or `json`. |

## Variables the server does not read

These variables appear in the repository, but the running server ignores them:

| Variable | Used by | Description |
| --- | --- | --- |
| `OXISTREAM_GIT_SHA` | Image build (`--build-arg`) | Commit shown on the admin status page. Compiled into the binary. |
| `OXISTREAM_IMAGE`, `OXISTREAM_PUBLISH` | [Compose](install/compose.md) | Image to run, and the host port binding. |
| `OXISTREAM_TEST_DATABASE_URL` | Backend tests | Database for integration tests, see [Development](development.md#database-for-tests). |
| `OXISTREAM_UPDATE_OPENAPI` | Backend tests | Rewrites `backend/openapi.json`, see [Development](development.md#api-types). |
