# Oxistream documentation

Oxistream is a self-hosted YouTube frontend with accounts, subscriptions, playlists, history and watch parties. All media is streamed through the instance.

## Using Oxistream

- [User guide](user-guide.md): home, search, the player and its shortcuts, queue, shorts, channels, subscriptions, playlists, history, watch parties, import and export, settings, and YouTube-compatible URLs.

## Running an instance

| Page | Contents |
| --- | --- |
| [Docker](install/docker.md) | Published images, `docker run`, image details, building the image |
| [Docker Compose](install/compose.md) | Setup with PostgreSQL, upgrades and backups |
| [Kubernetes](install/kubernetes.md) | Helm chart, CloudNativePG, values, multiple replicas |
| [Configuration](configuration.md) | Every `OXISTREAM_*` variable |
| [Reverse proxy](reverse-proxy.md) | Public URL, cookies, Origin check, client IPs, WebSockets; Caddy, nginx and Traefik examples |
| [Authentication](authentication.md) | Local accounts, first admin, require-login, OIDC with group-based admins, back-channel logout |
| [Administration](administration.md) | User management and the status page |
| [Operations](operations.md) | Upgrades, backups, multiple replicas, logs, health checks, metrics, rate limiting, troubleshooting |
| [Privacy](privacy.md) | What is proxied, what reaches YouTube and SponsorBlock, what is stored |

## Developing

| Page | Contents |
| --- | --- |
| [Development](development.md) | Dev shell, running locally, tests, quality gates, fixtures |
| [Architecture](architecture.md) | Crates, request flow, DASH, stream proxy, feed worker, watch party sync, app-wide player |
| [API](api.md) | OpenAPI and Scalar, versioning, authentication, watch party WebSocket protocol |
| [Releasing](releasing.md) | Versioning, release workflow, image tags |
