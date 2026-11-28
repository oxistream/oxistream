# Oxistream

Self-hosted, privacy-friendly YouTube frontend inspired by Invidious and Piped. Multi-user, aimed at homelabbers, also suitable for public hosting.

- `backend/`: Rust. Auth, subscriptions, playlists, history, YouTube data via RustyPipe, DASH manifest generation, stream proxy. Serves the embedded frontend.
- `frontend/`: SvelteKit SPA, embedded into the Rust binary.

## Ground rules

- Repo, code, docs, commits: English.
- Comments only where non-obvious logic needs explaining. No long comments or prose.
- Always use the latest stable versions of dependencies; verify on crates.io/npm before adding, never assume from memory.
- User, operator and developer docs live in `docs/`; update them with every user-visible or config change.
  - MkDocs with Material (`mkdocs.yml`, versions pinned in `requirements-docs.txt`, MkDocs 1.x only), landing page from `overrides/home.html`, built with `mkdocs build --strict` and deployed to GitHub Pages via Actions (`pages.yml`). Links to files outside `docs/` are absolute GitHub URLs.
- License: AGPL-3.0-or-later (required by RustyPipe's GPL-3.0). Do not add dependencies with incompatible licenses.

## Git workflow

- Conventional Commits 1.0: `type(scope): subject`, lowercase, imperative.
  - Types: `feat` `fix` `refactor` `perf` `docs` `test` `build` `ci` `chore` `style`
  - Scopes: `backend` `frontend` `dash` `db` `api` `auth` `player` `proxy` `deps` `repo` `deploy`
  - Breaking: `type(scope)!:` plus `BREAKING CHANGE:` footer.
  - Never add a `Co-Authored-By` trailer.
- Linear history only. No merge commits, no squash.
- Planning lives on GitHub: one milestone per phase, one issue per task, labels `area/backend|frontend|dash|auth|infra`.
- Every task: sub-agent in its own git worktree, branch `type/<issue>-short-name`, pushed, PR with `Closes #<issue>`.
- Merge via "Rebase and merge" only, after:
  - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
  - `pnpm lint`, `pnpm check`, `pnpm test`
  - green CI, diff reviewed, UI changes checked in a browser (light/dark, en/de)

## Product scope

MVP:
- Search, watch page (DASH player, description, related), channel pages
- Subscriptions with feed, import (YouTube Takeout CSV, NewPipe, Invidious, Piped) and export (NewPipe/Invidious JSON)
- Local playlists, visibility `private` or `unlisted` (share link)
- Watch history, resume, continue watching; pause, search and clear history; retention
- Accounts, account self-deletion, minimal admin (user list with role/disable/delete, status page)
- Discovery: home page (continue watching, subscriptions; trending for guests), search and channel tabs, shorts player, follow or import YouTube playlists
- SponsorBlock: segment markers, auto-skip or skip button per category

Later: comments, DeArrow, livestreams (HLS passthrough), SQLite.

Routes mirror YouTube so redirect tools work: `/watch?v=`, `/channel/UC…`, `/@handle`, `/playlist?list=`, `/results?search_query=`, `/shorts/{id}`, `/{videoId}`.

## Backend

- Cargo workspace, edition 2024. Crates:
  - `oxistream-server`: axum app, routes, feed worker
  - `oxistream-dash`: pure MPD generator, no web or IO deps
  - `oxistream-db`: sqlx repositories behind traits
  - Add crates only when justified.
- Stack: axum, tokio, tower-http, reqwest, serde, thiserror, `tracing`.
- API under `/api/v0` until first release, then `/api/v1`. OpenAPI via `utoipa`, Scalar UI at `/api/docs`.
- Config exclusively via `OXISTREAM_*` environment variables. No config files.
- Observability: `tracing` (JSON output via ENV), Prometheus `/metrics`, `/healthz`, `/readyz`.
- Dates: `time` (sqlx 0.9 has no native `jiff` support).

### Database

- PostgreSQL only for now (sqlx, migrations in `backend/migrations`). Minimum version 18; CI also tests 19.
- Repositories behind traits so SQLite can be added later.
- Tables:
  - `users`, `user_identities` (OIDC `(issuer, sub)`), `sessions` (incl. OIDC `sid` and ID token), `user_settings` (codec/quality prefs, history paused, feed filters), `oidc_logout_jti`
  - `videos`: metadata upsert cache (no media)
  - `channels`, `subscriptions`, `channel_videos` (feed), `feed_tokens` (Atom feed URL, SHA-256 only), `hidden_videos` ("Not interested"), `subscription_groups`, `subscription_group_channels`
  - `playlists`, `playlist_items` (ordered)
  - `queues`, `queue_entries` (up-next queue of signed-in users, ordered)
  - `youtube_playlists`, `playlist_follows`, `youtube_playlist_items` (followed YouTube playlists)
  - `watch_progress`: user, video, position, duration, finished, updated_at; doubles as history and resume point
  - `watch_rooms`, `watch_room_queue`, `watch_room_participants` (watch together)

### Auth

- Local accounts (argon2id) and OIDC, each toggleable via ENV. Registration toggleable. Without an OIDC admin group, a local registration becomes admin while no active (non-disabled) admin exists (users table lock); with one, admins come only from the IdP and local registrations stay `user`.
- Exactly one OIDC provider, by design. Authorization code flow with PKCE (`openidconnect`), auto-provisioning, identity is `(issuer, sub)`, never email. Admin role via configurable groups claim.
  - Enabled iff issuer and client ID are set; then `OXISTREAM_PUBLIC_URL` is required (redirect URI `{PUBLIC_URL}/api/v0/auth/oidc/callback`, post-logout redirect `{PUBLIC_URL}/`). Discovery runs in the background with backoff; `/readyz` fails until it succeeds.
  - ENV: `OXISTREAM_OIDC_ISSUER`, `OXISTREAM_OIDC_CLIENT_ID`, `OXISTREAM_OIDC_CLIENT_SECRET` (omit for public clients), `OXISTREAM_OIDC_SCOPES` (default `openid profile email`), `OXISTREAM_OIDC_NAME` (button label, default `SSO`), `OXISTREAM_OIDC_ADMIN_GROUP`, `OXISTREAM_OIDC_GROUPS_CLAIM` (default `groups`, dotted paths like `realm_access.roles` work).
  - `user_identities (issuer, subject) → user_id` links accounts; never linked to existing local accounts. OIDC users have no password. Username from `preferred_username`, else `name`, email local part, `sub`; sanitized, suffixed `-2`, `-3`… when taken.
  - Pending login (state, nonce, PKCE verifier, redirect) is an HMAC-signed `oxistream_oidc` cookie keyed with `OXISTREAM_PROXY_SECRET`, so any replica can finish it.
  - OIDC provisioning never applies the first-user rule; the registration toggle does not block it. With an admin group set, new users get their role from the groups claim (ID token, else userinfo) and it is synced on every login, also demoting the last admin (the IdP decides). Without one, OIDC users are provisioned as `user` and never synced, so promotions in the admin area stick. OIDC enabled with `OXISTREAM_AUTH_LOCAL_ENABLED=false` requires `OXISTREAM_OIDC_ADMIN_GROUP` (startup fails otherwise). Disabled users cannot sign in.
  - `OXISTREAM_OIDC_ADMIN_GROUP` must match a claim value exactly (Keycloak full-path groups: `/admins`).
  - Sessions store the `sid` claim and the ID token; `POST /auth/logout` returns `end_session_url` (with `id_token_hint`) when the provider has an end-session endpoint.
- Back-channel logout: validate the logout token manually against JWKS. Check `iss`, `aud`, `iat`, the `events` claim, presence of `sid` or `sub`, and absence of `nonce`; store `jti` to reject replays. Then end matching sessions.
  - `POST /api/v0/auth/oidc/backchannel-logout` (form `logout_token`), exempt from the Origin check and `OXISTREAM_REQUIRE_LOGIN`. Configure the IdP client with back-channel logout URL `{PUBLIC_URL}/api/v0/auth/oidc/backchannel-logout` and "session required" (so tokens carry `sid`).
  - Signature algs from discovery; unknown `kid` refetches the JWKS (throttled). `typ` must be `logout+jwt` if present; `iat` at most 10 min old (60 s leeway). `sid` ends that session (of `sub` if given), `sub` alone ends all of the user's sessions. Recording the `jti` and ending sessions share one transaction, so a failed attempt can be retried with the same token. 200 with `Cache-Control: no-store`, 400 OAuth error on invalid or replayed tokens; `oidc_logout_jti` rows expire after the acceptance window and are cleaned up hourly.
- Server-side sessions in DB; httpOnly, Secure, SameSite=Lax cookie; Origin check. No JWT sessions.
  - Cookie `oxistream_session`: random 256-bit token, only its SHA-256 is stored. Sliding expiry (`OXISTREAM_SESSION_TTL_DAYS`, default 30), renewed at most every 5 min.
  - Non-GET requests need `Origin` (or `Referer`) matching the `Host` header or `OXISTREAM_PUBLIC_URL` (optional, for proxies that rewrite `Host`), else 403.
  - ENV: `OXISTREAM_AUTH_LOCAL_ENABLED`, `OXISTREAM_REGISTRATION_ENABLED` (both default `true`), `OXISTREAM_COOKIE_SECURE` (default `true`, `false` only for plain-HTTP dev).
- `OXISTREAM_REQUIRE_LOGIN` (default `false`): anonymous users may search and watch. Subscriptions, playlists and history need an account; anonymous progress lives in localStorage.
  - Party guests: `/api/v0/rooms/{id}`, `…/ws` and `…/guest` stay open. A joined guest gets the `oxistream_party` cookie from `POST /rooms/{id}/guest` (`guest_key`; room id, participant key and confirmation time, HMAC with `OXISTREAM_PROXY_SECRET`, httpOnly), renewed every 30 s; `DELETE` clears it. With it the middleware lets anonymous-allowed requests through while the room exists, the key is not banned and the participant is connected (presence cached 5 s per replica), or was confirmed within the last 2 min (reloads, reconnects). Account-only endpoints stay 401.
  - SPA: the invite page and a tab that is (re)joining as guest are not locked; once out of the party it sends to sign-in, kicked or ended parties first to the invite page with the reason and sign-in actions. Leaving asks for confirmation then.

### Admin

- `/api/v0/admin/*` via the `AdminUser` extractor: 401 anonymous, 403 (`forbidden`) for non-admins. Frontend `/admin` (users) and `/admin/status`; the sidebar entry shows for admins only.
- Users: paginated list (`q`, `limit`, `offset`; newest first) with auth method, last seen (latest session), subscription count. `PATCH /admin/users/{id}` sets `role` / `disabled`, `DELETE` reuses the account deletion. Admins cannot change their own account there (409 `own_account`); the last active admin is never demoted or disabled (409 `last_admin`, under the users table lock). Disabling ends all sessions. OIDC group sync may change the role again on next sign-in, even of the last admin.
- Status: version (+ `OXISTREAM_GIT_SHA` at build time), uptime, DB reachability and version, counts, feed worker (lock held anywhere = running; due/pending/failing; last run of this replica's worker), player cache entries, botguard, auth/OIDC discovery, SponsorBlock, trending region, rate limiting.

### Watch history

- `GET /api/v0/history?q=` searches title and channel name (case-insensitive substring, `LIKE` wildcards escaped), same pagination; the UI debounces and keeps `?q=` in the URL.
- Retention `forever|365d|180d|90d|30d|14d|7d|1d` per user (`user_settings.history_retention_days`, NULL = forever) via `/history/settings`. `OXISTREAM_HISTORY_MAX_RETENTION` (same values, default `forever`, invalid fails startup) caps it: effective = min(user, max), higher values are rejected (422), stored ones above it act as the max, the UI offers only values up to it. `GET /api/v0/instance` reports `history_max_retention`; guests' localStorage progress is trimmed to it.
- Entries older than the effective retention (by `updated_at`) are filtered from all history queries and deleted hourly by any replica (advisory lock, idempotent).
- Video menus (signed in only): "Mark as watched" (`PUT /history/{id}`: finished, position = duration from the entry, the request or the metadata caches, else 422; recorded even while history is paused; not offered for shorts, live or upcoming) and "Mark as unwatched" (`DELETE /history/{id}`, removes entry and resume point).
- "Not interested" (`PUT`/`DELETE /hidden-videos/{id}`, undo toast) is offered only in the subscription feed and home rows; hidden videos leave the feed (also Atom and shorts seeds), "Continue watching" and trending, not history, search, related or channels.

### Subscription feed

- `GET /api/v0/feed?kind=&hide_watched=&channel=` filters server-side, so pages stay full; hidden videos are always left out, `hide_watched` drops finished videos. "Hide watched" and "Hide shorts" (no shorts shelf) are `user_settings.feed_hide_*` via `PATCH /settings`, so they follow the account; the channel filter is `?channel=` in the page URL, so it can be linked.
- Subscription groups (like NewPipe's channel groups): `/api/v0/subscription-groups` CRUD, `PUT …/{id}/position`, `PUT`/`DELETE …/{id}/channels/{channel_id}`. Many-to-many; members must be subscribed (FK to `subscriptions`, so unsubscribing or blocking drops them). At most 50 per user, names 1–50 chars, unique per user ignoring case (422 `taken`). Not part of import/export.
- `?group=` filters the feed (404 for other users' groups); the feed page shows group tabs (`?group=` in the URL, a deleted group falls back to all), the channels page has the same tabs, a per-channel group picker and a dialog to create, rename, reorder and delete groups. Groups with `show_on_home` get a home row after the subscriptions row (at the end without it); empty ones are hidden.

### YouTube data

- RustyPipe via `oxipipe`, our maintained fork (upstream is inactive), used as `rustypipe = { package = "oxipipe", git, rev }`. YouTube-side fixes (deobfuscation, clients, parsing) go into the fork, then bump the pinned `rev`.
- `.unauthenticated()` only. Never use YouTube accounts or cookies.
- Live YouTube tests (`tests/youtube_live.rs`, feature `live-tests`) run nightly and on demand (`live.yml`), not in PR checks.
- Fixtures from live responses must be scrubbed before committing: no IP addresses (e.g. googlevideo `ip=`), `visitorData` set to `redacted`, tracking params dropped. Grep the diff for IPv4 patterns before every push.
- Player clients: VisionOS first, TV as fallback; WEB is SABR-only. TV streams may only become valid after a pre-roll ad, so the proxy waits for `VideoPlayer::available_at`.
- `rustypipe-botguard` provides PO tokens. Bundled in the image, path overridable via ENV.
- Player responses cached in memory (`moka`) per instance. On an upstream 403 the proxy re-fetches player data.
- Trending: YouTube dropped its trending page and YouTube Charts needs reCAPTCHA, so `GET /api/v0/trending?category=podcasts|gaming|news|music|sports|learning|fashion|live` (default `podcasts`) uses the destination pages of `OXISTREAM_TRENDING_REGION` (ISO country code, default `US`, validated at startup), cached in memory per instance for 30 min. Each is a home row (guest default: news, gaming, podcasts). Left out: Shopping and Courses (redirect logged-out clients to the start page), Movies & TV (paid titles only), Playables (games).
- Shorts discovery: `GET /api/v0/shorts/discover` pages through the unauthenticated reel watch sequence (`reel/reel_watch_sequence`). Seeds: liked shorts and recent subscription shorts for signed-in users, plus trending; only seed video IDs reach YouTube. Each page continues the current sequence and starts the next seed, interleaved; blocked channels are left out. `GET /api/v0/shorts/{id}` (`reel/reel_item_watch`) returns the full video a short links to. The frontend skips shorts already shown in the browser session.
- Feed worker: interval via `OXISTREAM_FEED_INTERVAL_MINUTES` (default 30); fetches only due channels on a 1 min tick. Each channel is fetched once, however many users subscribe. A new subscription triggers an immediate fetch. Followed YouTube playlists work the same way; only their first 500 items are stored, for "new videos" counts. Runs under a Postgres advisory lock so multiple replicas are safe.
- New uploads: `subscriptions.last_visited_at` is set on subscribe, on a channel's first fetch (so its backlog is not new) and by `PUT /api/v0/subscriptions/{id}/visit` from the channel page. `has_new` in the subscription list: a feed video with `sort_at` after it, not upcoming, not hidden. The sidebar lists those channels first with a blue dot, then by name.
- Atom feed: `GET /api/v0/feed/atom/{token}` lists the latest 50 subscription videos; the token is the only credential (exempt from `OXISTREAM_REQUIRE_LOGIN`, redacted in request logs, 404 for disabled users). `POST /api/v0/settings/feed-token` returns the URL once and revokes the previous one, `DELETE` revokes. Links use `OXISTREAM_PUBLIC_URL`, else the request's `Host`; thumbnails go through the image proxy.

### Stream proxy

- All video, audio, thumbnail and subtitle traffic goes through our own proxy. Stream URLs are bound to the egress IP that fetched them; deployments should use a single egress IP.
- URLs are stateless, HMAC-SHA256-signed tokens (`exp` + `sig` query params) so any replica serves any request. Key from `OXISTREAM_PROXY_SECRET` (≥ 32 chars, e.g. `openssl rand -base64 48`), shared across replicas.
  - `/proxy/stream/{video_id}/{itag}?track=`, `/proxy/subtitle/{video_id}/{id}`: resolved against the replica's own player cache, so googlevideo URLs are only used from the IP that fetched them. Expire after hours.
  - `/proxy/image/{base64url(upstream)}`: expiry rounded up to the day so browsers can cache.
- Upstream host allowlist, also checked on redirects: `*.googlevideo.com` (streams), `*.ytimg.com`, `*.ggpht.com`, `yt3.googleusercontent.com` (images), `*.youtube.com` (subtitles).
- Range requests are passed through and streamed without buffering. No client cookies or IP headers upstream.
- Rate limiting via `tower_governor` on `/api` and `/proxy`, per client IP (trusted proxy header opt-in).

### DASH manifests

RustyPipe returns no MPD for regular videos, so `oxistream-dash` generates one per request:
- Static MPD with a single Period, `SegmentBase` using `init_range`/`index_range`.
- One AdaptationSet per mime + codec, and per audio track (`AudioTrack` id/lang/is_default).
- Audio roles from the `acont` xtag (`main`, `dub`, `description`); auto-dubs get an extra `<Role schemeIdUri="urn:oxistream:dash:role:2026" value="auto-dub"/>`.
- Include all streams; skip streams without ranges (OTF).
- All BaseURLs point to the proxy.
- Subtitles are not in the MPD; they are side-loaded as VTT (`&fmt=vtt`) via the proxy.
- Snapshot-tested with `insta` against fixture data.
- References: Invidious `manifest.cr`, Piped `DashUtils.js`, YouTube.js `DashManifest.tsx`.

### SponsorBlock

- Server-side only: clients never contact SponsorBlock. `GET /api/v0/videos/{id}/segments` asks `/api/skipSegments/{first 4 hex chars of sha256(id)}` for all categories (action types `skip`, `poi`, `full`) and keeps only the requested video. Generic User-Agent, no user data.
- Cached in memory (`moka`) per video: 1 h, 15 min when empty, 1 min after errors. Upstream errors yield no segments and a warning, never a failed request.
- Preferences per skip category `auto | button | off` (default: `sponsor` auto, the rest button), plus highlight marker and full-video label toggles. Stored in `user_sponsorblock_settings` (JSONB, missing keys take defaults) via `GET/PUT /api/v0/settings/sponsorblock`; guests use localStorage.
- Player: markers on the progress bar, category label on hover/scrub, auto-skip with undo, skip button inside `button` segments, "jump to highlight" button in the control bar; a segment the user seeks into or undoes is not auto-skipped again. Shorts only auto-skip.
- Smooth skips: ~8 s before an auto-skip (and while a skip button shows), the fragments dash.js will request at the target (from its parsed sidx/WebM cues, current video and audio representation) are fetched and handed to dash.js as blob URLs via a request interceptor; any mismatch goes to the network. No Service Worker.
- ENV: `OXISTREAM_SPONSORBLOCK_ENABLED` (default `true`), `OXISTREAM_SPONSORBLOCK_API_URL` (default `https://sponsor.ajay.app`). `GET /api/v0/instance` reports `sponsorblock`.

### Watch together

- Watch parties (backend: rooms). Invite link `/party/{id}`, ids are 128-bit random; joining opens the watch page of the current video. Only signed-in users start parties (header party button or the watch page); anyone with the link joins. Guests pick a name (kept in localStorage) and send a random guest key so kicks stick; with `OXISTREAM_REQUIRE_LOGIN` they may use the app only while in the party (see Auth).
- App-wide and per tab (`lib/together/party.svelte.ts`, id in sessionStorage to survive reloads): the party claims the app player, so its video plays in the watch page slot or the mini player while members browse. When the video changes, only someone on the old video's watch page is navigated to the new one. Other watch pages show the thumbnail with "Play now" (for everyone: `add` with `play: true`, or `select` if already queued) and "Add to queue" instead of their player. A party video that cannot play (unavailable, livestream) shows an error card in the slot or mini player. The mini player's X leaves after a confirmation.
- Header party button with member count toggles the party panel: invite link, queue, history, members, leave, end (creator). From 64rem it is a right sidebar next to the inset body card (open state in localStorage, the card and mini player make room); narrower, an off-canvas sheet. Joining via link opens it. "Add to party" on watch pages and in video menus.
- Host settings in the party menu (creator only, `watch_rooms` columns, bump the revision): who may control (play, pause, seek, skip, pick, reorder, shuffle, autoplay, remove others' entries) and who may add, each `everyone | host`; play now needs both. The repository checks every op in the room's transaction (`forbidden` error); others may always remove their own entries (`added_key`) and report `ended` within 5 s of the end. The UI hides what one may not do; without control, the player still follows and snaps back instead of sending. The queue autoplays. Only the creator kicks and ends.
- Chat tab: plain text up to 500 chars, whitespace collapsed, control and bidi characters dropped, rate limited per connection; unread badge on the tab and a dot on the party button. The latest 100 messages live in `watch_room_messages` (cascade with the room), sent as `chat_history` on join; a trigger notifies `watch_room_chat` with the whole message, without a new revision. Reactions (6 fixed emoji, throttled) are only notified on `watch_room_reaction` and float over the player (fade only with reduced motion). No watch progress is recorded. SponsorBlock never auto-skips in parties: segments only show the skip button, and a skip is a seek for everyone.
- Now-playing card: progress bar and time from the room state, for everyone. Autoplay toggle (party-wide, any member): once the current entry is the last, each replica with participants may look up its related videos; `Pick` stores the first regular video the room never queued (`watch_room_seen`, filled by a trigger) for that entry, compare-and-set, so everyone sees the same "Up next". When the entry ends, `Advance` appends and plays it (an `autoplay` entry). Queuing the pick by hand drops it; failed lookups retry after 1 min.
- Server-authoritative state in Postgres: current entry, `playing`, `position_ms` + `position_at`, `revision`. Clients ignore states with an older revision. `ended` is compare-and-set on the entry, so the queue advances once.
- Queue ops: `add`, `add_many` (≤ 100 IDs, as many as fit, answered with `added {added, skipped}`; uncached videos cost a rate-limit token each), `move {entry, to, revision}` (index among upcoming entries; a newer room answers `stale` and changes nothing), `shuffle`, `select`, `remove`. Entries are ordered by `position`; moves and shuffles only swap positions of upcoming entries.
- History: an entry that stops being current (ended, skipped, replaced, removed) is kept with a `played` sequence number instead of deleted; the last 100, one per video (playing one again replaces its older entry). It never counts for the queue limit, `select`, `move`, `shuffle` or `remove`. The server sends `history {entries}` (newest first) before the first `state` and whenever it changes, not with every state. History tab: click plays it again for everyone (`playNow`), menu queues it again.
- Party panel queue tab: search box (pasted link queues at once, text shows a few results), "From library" popover for signed-in users (continue watching, liked, own playlists, "queue all"), drag/keyboard reorder, shuffle.
- Sync: clients estimate the server clock NTP-style (median offset of the 3 fastest of the last 10 pings, burst on join, every 10 s, reset on reconnect) on `performance.now()`. `play`/`seek` carry `at` (server time the position was read, clamped to the last 5 s) so latency does not put others behind. Drift under 1 s is made up with `playbackRate` (P controller, ±5 %, pitch kept, 30/10 ms deadband); larger drift seeks ahead by a learned seek lead. Buffering is waited out; paused players seek beyond 40 ms. `?debug=sync` shows a readout.
- WebKit freezes playback ~300 ms on every `playbackRate` change. The controller learns that cost at runtime (drift vs. expected 1.5 s after a change; the least of the last 3, at least 2 needed, so one stall does not count; kept in localStorage); above 80 ms it catches up in rare phases at ±5 % (80 ms deadband) that end at `+cost`, so both freezes are accounted for. No user-agent checks.
- Seeks not made by the party sync (`party-sync.svelte.ts`) are the user's and reported on `seeking` (on release while dragging; the party is not followed while a pointer is down). Without pointer or key input on the player in the last second, seeks while stalled, within 3 s of our own seek or to the end count as dash.js, and seeks under 1 s are ignored.
- `position_at` and pongs use each replica's wall clock, so replicas must be NTP-synced (their skew adds to the drift).
- WebSocket `GET /api/v0/rooms/{id}/ws`: Origin check, join within 10 s, 4 KiB messages, per-connection token buckets, video ids validated. Limits: 10 rooms per user, 50 participants, 100 current and upcoming entries.
- Replicas sync via `LISTEN watch_rooms` (triggers notify the room id on every revision bump and delete) and reload the snapshot; a listener reconnect reloads all local rooms.
- Member status: clients send `status {sync, place}` on change (sync `synced|buffering|behind` from the party sync state, >1 s drift is `behind`, held 1 s before it counts; place `watching` on the party video's watch page, `shorts`, else `browsing`), at most every 2 s, again after reconnecting. The server stores the latest on the participant row at most once per second (no room revision bump); a trigger notifies `watch_room_status` (`room participant status_revision sync place`), and each replica forwards `status {participant, …, revision}` to its local members. States carry statuses too; the higher `status_revision` wins. Members tab: dot plus place, "Resync" (hard seek to the party position) on your own row. Join/leave notices float at the bottom of the panel for 5 s: your own join and rejoins within 5 s (same name) are skipped, bursts within 2 s are merged.
- Participants heartbeat every 30 s and are dropped after 2 min; rooms expire after 24 h without activity. Cleanup runs every 5 min on any replica.

## Frontend

- SvelteKit with `adapter-static` (SPA), Svelte 5 runes, Vite, Tailwind CSS 4, shadcn-svelte (bits-ui), `@lucide/svelte`, pnpm, Node LTS.
- API types generated from the backend OpenAPI with `openapi-typescript`. Requests via `openapi-fetch` and TanStack Svelte Query.
- i18n via Paraglide JS. English and German are required for every user-facing string.
- Player: Video.js 10 (`@videojs/html`) with `<dash-video>` (`@videojs/dash-video`, exact version pinned). Skin copied via shadcn and restyled. Codec and quality preferences come from user settings and are passed as dash.js settings. The initial audio track is the original, or a preferred language (`user_settings.audio_language`, localStorage for guests); auto-dubs never start unless picked in the menu. Progress is reported every ~10 s and on pause/unload; ≥ 90 % counts as finished.
- Chapters and transcript: video details carry `chapters` (title, start, thumbnails). The player gets them as an app chapters track (segmented progress bar, chapter title on hover). A collapsible panel on the watch page (side column; below the description on phones; open state and tab in localStorage `oxistream:watch-panel`) has tabs for the chapter list and the transcript. The transcript loads a subtitle track's VTT via the proxy (default: the captions showing, else the original language, else auto-generated), drops the rolled-over lines of auto-captions (karaoke timing marks new lines), is searchable (next/prev, Enter/Shift+Enter) and follows playback until the user scrolls. Clicks seek through `appPlayer.seek`; in a party only for members who may control it, others get no click targets instead of being snapped back.
- Up-next queue: "Play next" / "Add to queue" in video menus outside parties (in a party, "Add to party" takes their place). Edited in the mini player and in an "Up next" box on the watch page (drag handles plus keyboard and menu reordering). Shared by all tabs through localStorage `oxistream:queue` (`{owner, revision, entries}`, `storage` events); entries have random keys and every change goes by key, and taking an entry rereads storage first, so only the tab that advances takes the head and a second taker is a no-op. Signed-in users' queues live in `queues`/`queue_entries` (`GET /api/v0/queue`, `POST /queue/entries` with `position` and `skip_queued`, `DELETE /queue/entries/{key}`, `PUT /queue/entries/{key}/position`, `DELETE /queue`; every call returns the queue with its `revision`; 200 entries, client-sent metadata validated). Changes apply optimistically and are sent in order; the answer to the last one is shown unless storage holds a newer revision. Sign-in merges the guest queue (`skip_queued`), sign-out drops the user's copy; other devices' changes load on focus/visibility. When a video ends the queue plays first, then the playlist it interrupted. `appPlayer.next()` / `previous()` (restart after 3 s, else playlist or tab history) are the hooks for skip controls; a claim (party) replaces them via `onnext` / `onprevious`.
- App-wide player (`lib/player/app-player.svelte.ts`): one player rendered by the app shell, never moved in the DOM (Video.js tears down on disconnect). The watch page loads videos into it and marks a slot (`appPlayer.slot()`), which the player is positioned over; without a slot it is the mini player (bottom right, a bottom card on phones). Its controls (back to the video, close, play/pause, next) overlay the video on hover or keyboard focus, on a tap without hover; phones keep them in the bar. Hidden and paused on shorts pages; a party's video plays on there muted (`silenced`, not saved) in the mini player. `appPlayer.take(owner)` claims it exclusively (watch parties): pages cannot load other videos, and the mini player's close calls the holder.
- Player extras:
  - Shortcuts are Video.js hotkeys (YouTube keys; arrows ±5 s, j/l ±10 s), page-wide on the watch page; Shift+N/P (`appPlayer.next()`/`previous()`) and `?` (overlay listing them, `lib/player/shortcuts.ts`) are the app's. Arrow up/down, Home and End only while the player has focus, so the page still scrolls. The mini player only takes k and m page-wide; shorts keep their own set. Keys in fields and in dialogs outside the player stay there.
  - Media Session from the app player: title, channel, proxied thumbnail, play/pause/seek, next/previous only when `hasNext`/`hasPrevious`, position state. Cleared while hidden or silenced (shorts). In a party play/pause go through its sync like any input (non-controllers are snapped back); seeking is offered only when the claim's `mayControl()` allows it.
  - Settings menu: sleep timer (off, 15/30/45/60 min on the wall clock, end of video; kept across videos) and loop (native `loop`, so the queue does not advance; reset when another video loads). Loop and "end of video" exclude each other; both are hidden and reset in parties. Volume, mute, speed and captions are remembered per device (`oxistream:player`).
- Design: use the `design-taste-frontend` skill.
  - Light and dark mode, following the system by default.
  - Dense, content- and thumbnail-first, calm neutral surfaces.
  - Oxide-orange accent on neutral zinc. No YouTube red.
  - Vertical sidebar navigation based on shadcn-svelte `sidebar-08` (inset).
- Quality: ESLint + Prettier with Svelte plugins, vitest, Playwright E2E smoke tests, Storybook (`@storybook/sveltekit`, Svelte CSF) for components with light/dark and en/de toggles.

## Deployment

- One multi-stage Docker image: frontend build, Rust build, botguard binary.
  - Build stages run on `$BUILDPLATFORM`; the server is cross-compiled with Debian's cross gcc (no QEMU) for linux/amd64 and linux/arm64.
  - `rustypipe-botguard` is the upstream release binary, pinned by version and SHA-256 per arch (bump both together).
  - Runtime: `distroless/cc-debian13:nonroot`, no shell, no `HEALTHCHECK` (probe `/healthz`, `/readyz`). The server is PID 1 and handles SIGTERM.
  - Base images pinned by tag and digest. `OXISTREAM_GIT_SHA` is a build arg read at compile time; `VERSION` sets the OCI label.
  - CI builds the amd64 image without pushing.
- `deploy/compose/`: Compose setup with Postgres.
- `deploy/helm/oxistream`: Helm chart with example values for a CloudNativePG `Cluster`.
- CI via GitHub Actions; images published to GHCR.
  - `release.yml` on `vX.Y.Z` tags: checks the tag against the workspace version, `Cargo.lock` and `Chart.yaml` (`version`, `appVersion`), pushes the multi-arch image (SBOM, provenance, attestation) and the chart to `oci://ghcr.io/<owner>/charts`; manual runs default to a dry run.
