# Oxistream

Self-hosted, privacy-friendly YouTube frontend inspired by Invidious and Piped. Multi-user, aimed at homelabbers, also suitable for public hosting.

- `backend/`: Rust. Auth, subscriptions, playlists, history, YouTube data via RustyPipe, DASH manifest generation, stream proxy. Serves the embedded frontend.
- `frontend/`: SvelteKit SPA, embedded into the Rust binary.

## Ground rules

- Repo, code, docs, commits: English.
- Comments only where non-obvious logic needs explaining. No long comments or prose.
- Always use the latest stable versions of dependencies; verify on crates.io/npm before adding, never assume from memory.
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
- Watch history, resume, continue watching; pause and clear history
- Accounts, account self-deletion, minimal admin (user list with role/disable/delete, status page)

Later: comments, SponsorBlock, DeArrow, livestreams (HLS passthrough), YouTube playlist import, SQLite, shorts UI.

Routes mirror YouTube so redirect tools work: `/watch?v=`, `/channel/UC…`, `/@handle`, `/playlist?list=`, `/results?search_query=`, `/shorts/{id}` (redirects to watch), `/{videoId}`.

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
  - `users`, `user_identities` (OIDC `(issuer, sub)`), `sessions` (incl. OIDC `sid` and ID token), `user_settings` (codec/quality prefs, history paused), `oidc_logout_jti`
  - `videos`: metadata upsert cache (no media)
  - `channels`, `subscriptions`, `channel_videos` (feed)
  - `playlists`, `playlist_items` (ordered)
  - `watch_progress`: user, video, position, duration, finished, updated_at; doubles as history and resume point

### Auth

- Local accounts (argon2id) and OIDC, each toggleable via ENV. Registration toggleable. First user becomes admin.
- Exactly one OIDC provider, by design. Authorization code flow with PKCE (`openidconnect`), auto-provisioning, identity is `(issuer, sub)`, never email. Admin role via configurable groups claim.
  - Enabled iff issuer and client ID are set; then `OXISTREAM_PUBLIC_URL` is required (redirect URI `{PUBLIC_URL}/api/v0/auth/oidc/callback`, post-logout redirect `{PUBLIC_URL}/`). Discovery runs in the background with backoff; `/readyz` fails until it succeeds.
  - ENV: `OXISTREAM_OIDC_ISSUER`, `OXISTREAM_OIDC_CLIENT_ID`, `OXISTREAM_OIDC_CLIENT_SECRET` (omit for public clients), `OXISTREAM_OIDC_SCOPES` (default `openid profile email`), `OXISTREAM_OIDC_NAME` (button label, default `SSO`), `OXISTREAM_OIDC_ADMIN_GROUP`, `OXISTREAM_OIDC_GROUPS_CLAIM` (default `groups`, dotted paths like `realm_access.roles` work).
  - `user_identities (issuer, subject) → user_id` links accounts; never linked to existing local accounts. OIDC users have no password. Username from `preferred_username`, else `name`, email local part, `sub`; sanitized, suffixed `-2`, `-3`… when taken.
  - Pending login (state, nonce, PKCE verifier, redirect) is an HMAC-signed `oxistream_oidc` cookie keyed with `OXISTREAM_PROXY_SECRET`, so any replica can finish it.
  - First user becomes admin for OIDC too (same table lock as local registration); the registration toggle does not block provisioning. With an admin group set, the role is synced from the groups claim (ID token, else userinfo) on every login, but the last admin is never demoted. Disabled users cannot sign in.
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

### YouTube data

- RustyPipe via `oxipipe`, our maintained fork (upstream is inactive), used as `rustypipe = { package = "oxipipe", git, rev }`. YouTube-side fixes (deobfuscation, clients, parsing) go into the fork, then bump the pinned `rev`.
- `.unauthenticated()` only. Never use YouTube accounts or cookies.
- Player clients: VisionOS first, TV as fallback; WEB is SABR-only. TV streams may only become valid after a pre-roll ad, so the proxy waits for `VideoPlayer::available_at`.
- `rustypipe-botguard` provides PO tokens. Bundled in the image, path overridable via ENV.
- Player responses cached in memory (`moka`) per instance. On an upstream 403 the proxy re-fetches player data.
- Trending: YouTube dropped its trending page and YouTube Charts needs reCAPTCHA, so `GET /api/v0/trending?category=podcasts|gaming|news` (default `podcasts`) uses the destination pages of `OXISTREAM_TRENDING_REGION` (ISO country code, default `US`, validated at startup), cached in memory per instance for 30 min.
- Feed worker: interval via `OXISTREAM_FEED_INTERVAL_MINUTES` (default 30); fetches only due channels on a 1 min tick. Each channel is fetched once, however many users subscribe. A new subscription triggers an immediate fetch. Runs under a Postgres advisory lock so multiple replicas are safe.

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
- Include all streams; skip streams without ranges (OTF).
- All BaseURLs point to the proxy.
- Subtitles are not in the MPD; they are side-loaded as VTT (`&fmt=vtt`) via the proxy.
- Snapshot-tested with `insta` against fixture data.
- References: Invidious `manifest.cr`, Piped `DashUtils.js`, YouTube.js `DashManifest.tsx`.

## Frontend

- SvelteKit with `adapter-static` (SPA), Svelte 5 runes, Vite, Tailwind CSS 4, shadcn-svelte (bits-ui), `@lucide/svelte`, pnpm, Node LTS.
- API types generated from the backend OpenAPI with `openapi-typescript`. Requests via `openapi-fetch` and TanStack Svelte Query.
- i18n via Paraglide JS. English and German are required for every user-facing string.
- Player: Video.js 10 (`@videojs/html`) with `<dash-video>` (`@videojs/dash-video`, exact version pinned). Skin copied via shadcn and restyled. Codec and quality preferences come from user settings and are passed as dash.js settings. Progress is reported every ~10 s and on pause/unload; ≥ 90 % counts as finished.
- Design: use the `design-taste-frontend` skill.
  - Light and dark mode, following the system by default.
  - Dense, content- and thumbnail-first, calm neutral surfaces.
  - Oxide-orange accent on neutral zinc. No YouTube red.
  - Vertical sidebar navigation based on shadcn-svelte `sidebar-08` (inset).
- Quality: ESLint + Prettier with Svelte plugins, vitest, Playwright E2E smoke tests, Storybook (`@storybook/sveltekit`, Svelte CSF) for components with light/dark and en/de toggles.

## Deployment

- One multi-stage Docker image: frontend build, Rust build, botguard binary.
- `deploy/compose/`: Compose setup with Postgres.
- `deploy/helm/oxistream`: Helm chart with example values for a CloudNativePG `Cluster`.
- CI via GitHub Actions; images published to GHCR.
