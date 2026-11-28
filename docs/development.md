# Development

[Architecture](architecture.md) explains how the pieces fit together. Project rules (commit format, branch naming, review gates) are in [`CLAUDE.md`](../CLAUDE.md).

## Toolchain

With Nix:

```sh
nix develop
```

The shell provides stable Rust (with `rust-src` and `rust-analyzer`), Node 26, pnpm, sqlx-cli, cargo-insta and `rustypipe-botguard`. It exports `OXISTREAM_BOTGUARD_BIN` pointing at the botguard binary.

Without Nix, install these yourself:

- Rust stable, at least the workspace's `rust-version`
- Node LTS and pnpm (the version is pinned in `frontend/package.json`)
- Docker, for the backend database tests
- optionally `rustypipe-botguard` and sqlx-cli

## Running locally

Start a PostgreSQL 18 database, e.g. on port 5440:

```sh
docker run -d --name oxistream-dev-db -p 5440:5432 \
  -e POSTGRES_USER=oxistream -e POSTGRES_PASSWORD=oxistream -e POSTGRES_DB=oxistream \
  postgres:18
```

Run the backend on port 8080:

```sh
cd backend
OXISTREAM_DATABASE_URL=postgres://oxistream:oxistream@localhost:5440/oxistream \
OXISTREAM_PROXY_SECRET=dev-secret-dev-secret-dev-secret-dev \
cargo run -p oxistream-server
```

Run the frontend dev server in a second terminal:

```sh
cd frontend
pnpm install
pnpm dev
```

Open the URL Vite prints (`http://localhost:5173`). Vite proxies `/api` (including WebSockets), `/proxy` and `/healthz` to `http://localhost:8080`. Browsers accept `Secure` cookies on `localhost`, so `OXISTREAM_COOKIE_SECURE` can stay `true`.

To serve the built SPA from the backend instead, as in production:

```sh
(cd frontend && pnpm build)
(cd backend && cargo run -p oxistream-server --features embed-frontend)
```

Without `embed-frontend`, the backend answers non-API routes with 404.

## Quality gates

CI runs these, and each must pass before merging:

| Where | Command |
| --- | --- |
| `backend/` | `cargo fmt --check` |
| `backend/` | `cargo clippy --all-targets -- -D warnings` |
| `backend/` | `cargo test` (CI: `cargo test --features embed-frontend -- --include-ignored`) |
| `frontend/` | `pnpm lint` (Prettier and ESLint) |
| `frontend/` | `pnpm check` (svelte-check) |
| `frontend/` | `pnpm test` (Vitest) |
| `frontend/` | `pnpm test:e2e`, `pnpm test:storybook` |

CI also checks:

- that the generated API types are up to date,
- that `docker compose config` accepts the Compose file,
- the Helm chart, with `helm lint` and `helm template` validated by kubeconform,
- the backend against PostgreSQL 18 and 19.

UI changes are checked in a browser in light and dark mode, in English and German.

## Database for tests

Tests that need PostgreSQL are marked `#[ignore]` and run with `--include-ignored`:

```sh
cd backend
cargo test -- --include-ignored
```

They use `OXISTREAM_TEST_DATABASE_URL` if it is set. Otherwise they start a PostgreSQL 18 container through testcontainers, which needs a Docker daemon. Tests that need empty tables create their own schema.

The SQL is checked at runtime, not with sqlx compile-time macros, so building does not need a database.

### Migrations

Migrations live in `backend/migrations` and run at server startup. Add one with:

```sh
cd backend && sqlx migrate add <name>
```

Migrations only go forward. Do not edit a migration that has been released.

## Snapshot tests

DASH manifests (`oxistream-dash/tests`) and some API responses are snapshot-tested with [insta](https://insta.rs). After an intended change:

```sh
cargo insta test --review
```

## API types

The backend's OpenAPI document is committed as `backend/openapi.json`, and a test fails when it is out of date. After changing the API:

```sh
cd backend && OXISTREAM_UPDATE_OPENAPI=1 cargo test --test openapi
cd ../frontend && pnpm gen:api
```

`pnpm gen:api` regenerates `frontend/src/lib/api/schema.d.ts` with openapi-typescript. `oxistream-server openapi` also prints the document.

## Frontend

| Command | Purpose |
| --- | --- |
| `pnpm dev` | Dev server with the API proxy |
| `pnpm build` | Static SPA in `frontend/build` |
| `pnpm format` | Prettier |
| `pnpm i18n` | Compile Paraglide messages (run by `check` and `prepare`) |
| `pnpm test:e2e` | Playwright against a production build on port 4173. The API is mocked per test with `page.route`, so no backend is needed. On macOS, the watch party sync tests also run in WebKit. |
| `pnpm storybook` | Storybook on port 6006, with light/dark and en/de toolbar toggles |
| `pnpm test:storybook` | Builds Storybook and smoke-tests every story with Playwright |

Install the browsers once with `pnpm exec playwright install chromium` (and `webkit` on macOS).

Every user-facing string goes into both `frontend/messages/en.json` and `frontend/messages/de.json`.

## Live YouTube tests

`backend/crates/oxistream-server/tests/youtube_live.rs` talks to real YouTube. It is behind the `live-tests` feature and kept out of PR checks, because results depend on what YouTube serves the runner:

```sh
cd backend
cargo test -p oxistream-server --features live-tests --test youtube_live -- --include-ignored
```

The `Live YouTube` workflow runs it nightly and on demand.

## Fixtures

Fixtures recorded from live YouTube responses must be scrubbed before you commit them:

- no IP addresses (e.g. the `ip=` parameter of googlevideo URLs),
- `visitorData` set to `redacted`,
- tracking parameters removed.

Grep the diff for IPv4 addresses before every push:

```sh
git diff origin/main | grep -nE '\b([0-9]{1,3}\.){3}[0-9]{1,3}\b'
```

## RustyPipe fork

YouTube access goes through `oxipipe`, a maintained fork of RustyPipe. It is pinned by git `rev` in `backend/Cargo.toml` as `rustypipe = { package = "oxipipe", ... }`. Fix YouTube-side breakage (deobfuscation, clients, parsing) in the fork, then bump the `rev`.
