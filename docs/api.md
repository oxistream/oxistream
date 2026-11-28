---
title: API
parent: Development
nav_order: 2
---

# API

Every running instance documents its own HTTP API:

| URL | Content |
| --- | --- |
| `/api/docs` | Scalar UI |
| `/api/openapi.json` | OpenAPI document |

The same document is committed as [`backend/openapi.json`](../backend/openapi.json), and `oxistream-server openapi` prints it. The frontend's types are generated from it, see [Development](development.md#api-types).

## Versioning

The API lives under `/api/v0` until the first stable release, and will move to `/api/v1` then. Until then, `v0` may change in any release. The frontend shipped in the same binary always matches.

## Requests

- JSON request and response bodies, except for a few endpoints:
  - `manifest.mpd` (DASH XML),
  - `storyboard.vtt` (WebVTT),
  - `feed/atom/{token}` (Atom),
  - subscription import (raw file body) and export (file download).
- Paginated lists use opaque cursors.
- Errors share one shape:

  ```json
  { "code": "validation_failed", "message": "...", "fields": { "username": "format" } }
  ```

  `code` is machine-readable. `reason` adds detail where useful, e.g. why a video is unavailable.

- `GET /api/v0/instance` needs no session, even with require-login. It reports the instance's capabilities: enabled sign-in methods, registration, require-login, the OIDC button label, SponsorBlock and the maximum history retention.

## Authentication

- Sessions use the `oxistream_session` cookie (security scheme `session` in the OpenAPI document). Get one with `POST /api/v0/auth/login` or `register`, or through the OIDC flow at `GET /api/v0/auth/oidc/login`. End it with `POST /api/v0/auth/logout`.
- `GET /api/v0/auth/me` returns the current user.
- Non-GET requests need an `Origin` or `Referer` header of the instance, otherwise 403. See [Reverse proxy](reverse-proxy.md#origin-check). Non-browser clients have to send it themselves:

  ```sh
  curl -c jar -H 'Origin: https://oxistream.example.com' -H 'Content-Type: application/json' \
    -d '{"username":"alice","password":"correct horse battery"}' \
    https://oxistream.example.com/api/v0/auth/login
  ```

- Endpoints that need a user return 401 without a session. Admin endpoints return 403 for non-admins.
- `/api` and `/proxy` are [rate-limited](operations.md#rate-limiting) per client IP. Over the limit, they return 429 with `Retry-After`.

The Atom feed (`GET /api/v0/feed/atom/{token}`) uses its token instead of a session. Create the token with `POST /api/v0/settings/feed-token`. The full URL is returned only once.

## Proxy URLs

Responses never contain YouTube media URLs. They contain signed `/proxy/...` URLs with `exp` and `sig` parameters. Use them as they are: they expire, and changing them breaks the signature. See [Architecture](architecture.md#stream-proxy).

## Watch party WebSocket

`GET /api/v0/rooms/{id}/ws` upgrades to a WebSocket. Create a room first with `POST /api/v0/rooms` (signed in).

The handshake must carry an `Origin` of the instance. Guests may always connect, also when the instance requires login.

With require-login, a guest that joined calls `POST /api/v0/rooms/{id}/guest` with its `guest_key` to get the `oxistream_party` cookie, and again every 30 s. The cookie lets it use the API and the proxy while it is in the room. `DELETE /api/v0/rooms/{id}/guest` clears it. See [party guests](authentication.md#party-guests).

Messages are JSON text frames tagged by `type`. Positions are in seconds, and times in milliseconds since the Unix epoch. Both message unions are in the OpenAPI document as `RoomClientMessage` and `RoomServerMessage`.

### Session

1. The client sends `join` within 10 s. Signed-in users send `{"type":"join"}`. Guests add `name` (1–32 characters) and a random `guest_key` (16–128 URL-safe characters) that they keep, so that a removal sticks.
2. The server answers `welcome` (`participant_id`, `is_owner`, `server_time`), then `chat_history` and `history`, then `state`.
3. The server sends a new `state` (the whole room, with a growing `revision`) after every change. Ignore states with a lower revision than the last one applied.
4. The client sends `ping {t}` regularly and gets `pong {t, server_time}`, for clock estimation. It sends `status` when its player or page changes.
5. The connection ends with `kicked` or `closed`, or when the client goes away. Participants without a heartbeat for 2 min are dropped.

### Client messages

| Type | Fields | Notes |
| --- | --- | --- |
| `join` | `name?`, `guest_key?` | First message |
| `ping` | `t` | |
| `play` | `position`, `at?` | `at`: server time the position was read, at most 5 s back |
| `pause` | `position` | |
| `seek` | `position`, `at?` | |
| `ended` | `entry` | The queue advances once, however many clients send it |
| `skip`, `select`, `remove` | `entry` | |
| `add` | `video_id`, `play?` | `play: true` plays it now |
| `add_many` | `video_ids` | Up to 100. Answered with `added {added, skipped}` |
| `move` | `entry`, `to`, `revision` | `stale` error if the room changed since `revision` |
| `shuffle` | | |
| `autoplay` | `enabled` | |
| `kick` | `participant` | Owner only. Permanent for the room |
| `permissions` | `control?`, `add?` | Owner only. `everyone` or `host` |
| `chat` | `text` | Up to 500 characters |
| `react` | `reaction` | `heart`, `laugh`, `wow`, `sad`, `clap`, `fire` |
| `status` | `sync?`, `place` | `sync`: `synced`, `buffering`, `behind`. `place`: `watching`, `browsing`, `shorts` |

Under `permissions.control: host`, only the owner may use playback and queue control: `play`, `pause`, `seek`, `skip`, `select`, `move`, `shuffle`, `autoplay`, and removing other people's entries. Under `permissions.add: host`, only the owner may add videos.

### Server messages

`welcome`, `state`, `history`, `chat_history`, `chat`, `reaction`, `status`, `pong`, `added`, `error {code}`, `kicked`, `closed`.

Error codes: `not_found`, `room_full`, `queue_full`, `invalid`, `video_unavailable`, `forbidden`, `rate_limited`, `stale`, `internal`.

### Limits

| Limit | Value |
| --- | --- |
| Message size | 4 KiB |
| Messages per connection | burst 20, 10/s |
| `add`/`add_many`, chat | burst 5, one every 2 s |
| Reactions | burst 5, 1.5/s |
| Rooms per user | 10 |
| Participants per room | 50 |
| Queue entries | 100 |
| Room lifetime without activity | 24 h |
