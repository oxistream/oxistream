# Oxistream

A self-hosted, privacy-friendly YouTube frontend with accounts, subscriptions and playlists. Rust backend, SvelteKit frontend.

> Work in progress.

## Development

With Nix, `nix develop` provides the Rust toolchain, Node, pnpm, sqlx-cli and `rustypipe-botguard` (exported as `OXISTREAM_BOTGUARD_BIN`).

## Docker

Release images for linux/amd64 and linux/arm64 are published as `ghcr.io/OWNER/oxistream` with tags `X.Y.Z`, `X.Y`, `X` (from 1.0) and `latest`. Each carries SBOM and provenance attestations (`gh attestation verify oci://ghcr.io/OWNER/oxistream:X.Y.Z --owner OWNER`).

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

## Kubernetes

A Helm chart with CloudNativePG example values lives in [`deploy/helm/oxistream`](deploy/helm/oxistream/README.md). Releases also publish it as `oci://ghcr.io/OWNER/charts/oxistream`.

## Releasing

The workspace version in `backend/Cargo.toml` is the source of truth; `version` and `appVersion` in `deploy/helm/oxistream/Chart.yaml` follow it.

1. Bump all three, run `cargo update --workspace` in `backend/` to update `Cargo.lock`, and merge the change.
2. Tag the merge commit and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.

The `Release` workflow fails if the tag and these versions differ, then pushes the image and the chart to GHCR. Pre-release tags like `v1.2.0-rc.1` do not move `latest`. Run it manually with "dry run" to build a branch or tag without pushing. New GHCR packages start private; make them public once in the package settings.

## License

[AGPL-3.0-or-later](LICENSE)
