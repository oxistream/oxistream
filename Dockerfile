# syntax=docker/dockerfile:1

# Build stages run on the build host; the server is cross-compiled for TARGETARCH.

FROM --platform=$BUILDPLATFORM node:26.13.1-trixie-slim@sha256:abe7e5a82b61a8ef67b315b85497c9e65173387ccf2f4a029248184b8e020be5 AS frontend
# Node 25+ no longer bundles corepack.
RUN npm install --global corepack@0.36.0 && corepack enable
WORKDIR /src/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN corepack install
RUN --mount=type=cache,id=pnpm-store,target=/pnpm-store \
    pnpm install --frozen-lockfile --store-dir /pnpm-store
COPY frontend/ ./
RUN pnpm run prepare && pnpm build

FROM --platform=$BUILDPLATFORM rust:1.100.0-slim-trixie@sha256:3b199fd6db7d709274df56565b8f4003f9e30d7bfcff9d53148c86053b7855e2 AS backend
ARG BUILDARCH
ARG TARGETARCH
RUN set -eu; \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-gnu; gcc=gcc-x86-64-linux-gnu; libc=libc6-dev-amd64-cross ;; \
      arm64) triple=aarch64-unknown-linux-gnu; gcc=gcc-aarch64-linux-gnu; libc=libc6-dev-arm64-cross ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    if [ "$TARGETARCH" != "$BUILDARCH" ]; then \
      apt-get update; \
      apt-get install -y --no-install-recommends "$gcc" "$libc"; \
      rm -rf /var/lib/apt/lists/*; \
    fi; \
    rustup target add "$triple"; \
    echo "$triple" > /rust-target
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar \
    CARGO_PROFILE_RELEASE_STRIP=symbols \
    CARGO_TERM_COLOR=never
WORKDIR /src/backend
COPY backend/ ./
COPY --from=frontend /src/frontend/build /src/frontend/build
# Read at compile time by the admin status page.
ARG OXISTREAM_GIT_SHA=""
ENV OXISTREAM_GIT_SHA=$OXISTREAM_GIT_SHA
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target,target=/src/backend/target,sharing=locked \
    set -eu; \
    triple="$(cat /rust-target)"; \
    cargo build --locked --release --target "$triple" -p oxistream-server --features embed-frontend; \
    install -D "target/$triple/release/oxistream-server" /out/oxistream-server

FROM --platform=$BUILDPLATFORM debian:13.8-slim@sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0 AS botguard
ARG TARGETARCH
ARG BOTGUARD_VERSION=0.1.2
ARG BOTGUARD_SHA256_AMD64=4f2ec561e8f9fadece7deadc6ce0624fbdedd852222c3eb194c22153b1323129
ARG BOTGUARD_SHA256_ARM64=4d038857374a69aea9be8ded981d93a776dc88d4e254f5c6d292746099abf69a
RUN set -eu; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl xz-utils; \
    rm -rf /var/lib/apt/lists/*; \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-gnu; sha="$BOTGUARD_SHA256_AMD64" ;; \
      arm64) triple=aarch64-unknown-linux-gnu; sha="$BOTGUARD_SHA256_ARM64" ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    file="rustypipe-botguard-v${BOTGUARD_VERSION}-${triple}.tar.xz"; \
    curl -fsSLo "/tmp/$file" "https://codeberg.org/ThetaDev/rustypipe-botguard/releases/download/v${BOTGUARD_VERSION}/$file"; \
    echo "$sha  /tmp/$file" | sha256sum -c -; \
    tar -xJf "/tmp/$file" -C /tmp; \
    install -D /tmp/rustypipe-botguard /out/rustypipe-botguard; \
    install -d -o 65532 -g 65532 /out/data

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:29ee70762214fe8190cea04ef1c0a84a8365879e047ce8f911ae5a5ab2355fa2
ARG VERSION=dev
ARG SOURCE_URL=""
ARG OXISTREAM_GIT_SHA=""
LABEL org.opencontainers.image.title="Oxistream" \
      org.opencontainers.image.description="Self-hosted, privacy-friendly YouTube frontend" \
      org.opencontainers.image.source="${SOURCE_URL}" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.revision="$OXISTREAM_GIT_SHA"
COPY --from=botguard /out/rustypipe-botguard /usr/local/bin/rustypipe-botguard
COPY --from=botguard --chown=65532:65532 /out/data /var/lib/oxistream
COPY --from=backend /out/oxistream-server /usr/local/bin/oxistream-server
ENV OXISTREAM_BIND=0.0.0.0:8080 \
    OXISTREAM_BOTGUARD_BIN=/usr/local/bin/rustypipe-botguard \
    OXISTREAM_YOUTUBE_CACHE_DIR=/var/lib/oxistream
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/oxistream-server"]
