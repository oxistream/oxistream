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
  - `users`, `sessions` (incl. OIDC `sid`), `user_settings` (codec/quality prefs, history paused), `oidc_logout_jti`
  - `videos`: metadata upsert cache (no media)
  - `channels`, `subscriptions`, `channel_videos` (feed)
  - `playlists`, `playlist_items` (ordered)
  - `watch_progress`: user, video, position, duration, finished, updated_at; doubles as history and resume point

### Auth

- Local accounts (argon2id) and OIDC, each toggleable via ENV. Registration toggleable. First user becomes admin.
- Exactly one OIDC provider, by design. Authorization code flow with PKCE (`openidconnect`), auto-provisioning, identity is `(issuer, sub)`, never email. Admin role via configurable groups claim.
- Back-channel logout: validate the logout token manually against JWKS. Check `iss`, `aud`, `iat`, the `events` claim, presence of `sid` or `sub`, and absence of `nonce`; store `jti` to reject replays. Then end matching sessions.
- Server-side sessions in DB; httpOnly, Secure, SameSite=Lax cookie; Origin check. No JWT sessions.
- `OXISTREAM_REQUIRE_LOGIN` (default `false`): anonymous users may search and watch. Subscriptions, playlists and history need an account; anonymous progress lives in localStorage.

### YouTube data

- RustyPipe with `.unauthenticated()`. Never use YouTube accounts or cookies.
- `rustypipe-botguard` provides PO tokens. Bundled in the image, path overridable via ENV.
- Player responses cached in memory (`moka`) per instance. On an upstream 403 the proxy re-fetches player data.
- Feed worker: interval via ENV (default 30 min). Each channel is fetched once, however many users subscribe. A new subscription triggers an immediate fetch. Runs under a Postgres advisory lock so multiple replicas are safe.

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
