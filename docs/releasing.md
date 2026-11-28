# Releasing

The workspace version in `backend/Cargo.toml` is the source of truth. `version` and `appVersion` in `deploy/helm/oxistream/Chart.yaml` follow it.

## Steps

1. Bump the version in all three places, and update `Cargo.lock`:

   ```sh
   cd backend && cargo update --workspace
   ```

   Merge the change like any other PR.
2. Tag the merge commit and push the tag:

   ```sh
   git tag vX.Y.Z && git push origin vX.Y.Z
   ```

The `Release` workflow (`.github/workflows/release.yml`) then:

1. Checks that the tag, `Cargo.toml`, `Cargo.lock` and both chart versions agree. It fails otherwise.
2. Builds the image for `linux/amd64` and `linux/arm64`. The version and commit are passed as build arguments.
3. Pushes it to `ghcr.io/OWNER/oxistream` with SBOM and provenance attestations, and attests the build provenance.
4. Packages the Helm chart and pushes it to `oci://ghcr.io/OWNER/charts/oxistream`.

## Tags

| Tag | Example for `v1.2.3` |
| --- | --- |
| `X.Y.Z` | `1.2.3` |
| `X.Y` | `1.2` |
| `X` | `1`, only from 1.0 on |
| `latest` | not for pre-releases |
| `sha-<short commit>` | `sha-1a2b3c4` |

Pre-release tags such as `v1.2.0-rc.1` do not move `latest`.

## Dry runs

Run the workflow manually ("Run workflow") with a `ref` and "dry run" checked. It builds a branch, tag or commit without pushing anything. Without a semver tag, the version comes from `Cargo.toml`.

## First release

New GHCR packages start private. After the first release, make the image and the chart package public in the package settings.
