# Docker

Oxistream ships as a single image: the server with the frontend embedded, plus `rustypipe-botguard`. It needs PostgreSQL 18 or newer. For a ready-made setup with PostgreSQL, use [Docker Compose](compose.md).

## Images

Releases publish `ghcr.io/OWNER/oxistream` for `linux/amd64` and `linux/arm64`:

| Tag | Moves on |
| --- | --- |
| `X.Y.Z` | never |
| `X.Y` | every patch release |
| `X` | every minor release, from 1.0 on |
| `latest` | every release except pre-releases (`-rc.1` etc.) |
| `sha-<commit>` | never |

Each image carries SBOM and provenance attestations:

```sh
gh attestation verify oci://ghcr.io/OWNER/oxistream:X.Y.Z --owner OWNER
```

## Run

```sh
openssl rand -base64 48 > proxy-secret   # once; keep it
docker run -d --name oxistream -p 8080:8080 \
  -e OXISTREAM_DATABASE_URL=postgres://oxistream:secret@db:5432/oxistream \
  -e OXISTREAM_PROXY_SECRET="$(cat proxy-secret)" \
  -v oxistream-cache:/var/lib/oxistream \
  ghcr.io/OWNER/oxistream:X.Y.Z
```

Keep the proxy secret stable. If it changes, existing proxy links and pending SSO logins stop working. Open `http://localhost:8080`. The first account to register becomes admin.

The two variables above are the only required ones. See [Configuration](../configuration.md) for everything else. Behind a reverse proxy, also read [Reverse proxy](../reverse-proxy.md).

## Image details

| | |
| --- | --- |
| Base | `gcr.io/distroless/cc-debian13:nonroot`, no shell |
| User | `65532:65532` |
| Port | `8080` |
| Preset environment | `OXISTREAM_BIND=0.0.0.0:8080`, `OXISTREAM_BOTGUARD_BIN=/usr/local/bin/rustypipe-botguard`, `OXISTREAM_YOUTUBE_CACHE_DIR=/var/lib/oxistream` |
| Volume | `/var/lib/oxistream`: RustyPipe client cache and botguard snapshot. Cheap to rebuild, so it may be an anonymous volume or tmpfs. |
| Commands | `oxistream-server` (or `serve`) runs the server. `oxistream-server openapi` prints the OpenAPI document. |

The image works with a read-only root filesystem as long as `/var/lib/oxistream` and `/tmp` are writable.

The image has no `HEALTHCHECK` because it has no shell. Probe these endpoints from outside the container:

- `/healthz` for liveness. It always returns `ok`.
- `/readyz` for readiness. It checks the database and, if OIDC is enabled, provider discovery.

## Build it yourself

The root `Dockerfile` builds the frontend and the server, and downloads a pinned, checksum-verified `rustypipe-botguard`. The server is cross-compiled on the build host, so multi-arch builds need no emulation:

```sh
docker buildx build --platform linux/amd64,linux/arm64 \
  --build-arg VERSION=0.1.0 \
  --build-arg OXISTREAM_GIT_SHA="$(git rev-parse HEAD)" \
  -t oxistream .
```

| Build argument | Description |
| --- | --- |
| `VERSION` | `org.opencontainers.image.version` label. Default `dev`. |
| `OXISTREAM_GIT_SHA` | Commit, compiled in for the admin status page. |
| `SOURCE_URL` | `org.opencontainers.image.source` label. |
| `BOTGUARD_VERSION`, `BOTGUARD_SHA256_AMD64`, `BOTGUARD_SHA256_ARM64` | `rustypipe-botguard` release and checksums. |

## Upgrade

Pull the new tag and recreate the container. Migrations run at startup. See [Operations](../operations.md#upgrades).
