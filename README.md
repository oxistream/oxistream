# Oxistream

A self-hosted, privacy-friendly YouTube frontend with accounts, subscriptions and playlists. Rust backend, SvelteKit frontend.

> Work in progress.

## Development

With Nix, `nix develop` provides the Rust toolchain, Node, pnpm, sqlx-cli and `rustypipe-botguard` (exported as `OXISTREAM_BOTGUARD_BIN`).

## Docker

The root `Dockerfile` builds the frontend, the server with the embedded SPA, and bundles `rustypipe-botguard`. The server is cross-compiled on the build host, so multi-arch builds need no emulation:

```sh
docker buildx build --platform linux/amd64,linux/arm64 \
  --build-arg VERSION=0.1.0 --build-arg OXISTREAM_GIT_SHA="$(git rev-parse HEAD)" \
  -t oxistream .
```

Run it against PostgreSQL 18 or newer; migrations run at startup:

```sh
docker run -p 8080:8080 \
  -e OXISTREAM_DATABASE_URL=postgres://oxistream:secret@db:5432/oxistream \
  -e OXISTREAM_PROXY_SECRET="$(openssl rand -base64 48)" \
  oxistream
```

`OXISTREAM_DATABASE_URL` and `OXISTREAM_PROXY_SECRET` (at least 32 characters, shared across replicas) are required. Set `OXISTREAM_PUBLIC_URL` behind a reverse proxy, and `OXISTREAM_COOKIE_SECURE=false` only for plain-HTTP testing. The image runs as a non-root user on distroless without a shell, so there is no `HEALTHCHECK`; probe `/healthz` (liveness) and `/readyz` (database and OIDC discovery) instead. `/var/lib/oxistream` holds RustyPipe's client cache and may be mounted as a volume.

For a ready-made setup with PostgreSQL, see [`deploy/compose`](deploy/compose/README.md).

## License

[AGPL-3.0-or-later](LICENSE)
