# Setup

Oxistream is a single container image for `linux/amd64` and `linux/arm64`, next to a PostgreSQL database.

## Requirements

- PostgreSQL 18 or newer. CI also tests 19.
- A secret of at least 32 characters for `OXISTREAM_PROXY_SECRET`, e.g. from `openssl rand -base64 48`.
- An HTTPS [reverse proxy](../advanced/reverse-proxy.md) for anything beyond `localhost`.
- One egress IP towards YouTube. Stream URLs are bound to the IP that fetched them.

## Pick an installation method

<div class="grid cards" markdown>

-   :material-docker:{ .lg .middle } **Docker Compose**

    ---

    One host, PostgreSQL included, hardened containers. The quickest start.

    [:octicons-arrow-right-24: Docker Compose](compose.md)

-   :material-package-variant-closed:{ .lg .middle } **Docker**

    ---

    Run the image next to a database you already have. Image tags, details and how to build it.

    [:octicons-arrow-right-24: Docker](docker.md)

-   :material-kubernetes:{ .lg .middle } **Kubernetes**

    ---

    The Helm chart, optionally with CloudNativePG and several replicas.

    [:octicons-arrow-right-24: Kubernetes](kubernetes.md)

-   :material-transit-connection-variant:{ .lg .middle } **Reverse proxy**

    ---

    TLS in front of any of the three, with Caddy, nginx and Traefik examples.

    [:octicons-arrow-right-24: Reverse proxy](../advanced/reverse-proxy.md)

</div>

## Quick start

=== "Docker Compose"

    ```sh
    git clone https://github.com/oxistream/oxistream.git
    cd oxistream/deploy/compose
    cp .env.example .env
    # In .env, set POSTGRES_PASSWORD (openssl rand -hex 32)
    # and OXISTREAM_PROXY_SECRET (openssl rand -base64 48).
    docker compose up -d
    ```

=== "Docker"

    ```sh
    openssl rand -base64 48 > proxy-secret   # once; keep it
    docker run -d --name oxistream -p 8080:8080 \
      -e OXISTREAM_DATABASE_URL=postgres://oxistream:secret@db:5432/oxistream \
      -e OXISTREAM_PROXY_SECRET="$(cat proxy-secret)" \
      -v oxistream-cache:/var/lib/oxistream \
      ghcr.io/OWNER/oxistream:X.Y.Z
    ```

=== "Helm"

    ```sh
    helm install oxistream oci://ghcr.io/OWNER/charts/oxistream --version X.Y.Z \
      --set image.repository=ghcr.io/OWNER/oxistream \
      --set database.url='postgres://oxistream:secret@postgres:5432/oxistream' \
      --set config.OXISTREAM_PUBLIC_URL=https://oxistream.example.com
    ```

## First steps

1. Open the instance, e.g. `http://localhost:8080`, and register right away. Without an OIDC admin group, the first local account becomes the admin, also when SSO users signed in before, see [first admin](../advanced/authentication.md#first-admin). SSO users become admins only through the [admin group](../advanced/authentication.md#admin-role-from-groups).
2. Put a TLS-terminating [reverse proxy](../advanced/reverse-proxy.md) in front, and set `OXISTREAM_PUBLIC_URL`.
3. Decide who may use the instance: registration, require-login and SSO are in [Authentication](../advanced/authentication.md).
4. Look through [Configuration](../configuration.md) for the rest, and check the admin [status page](../administration/index.md#status).
