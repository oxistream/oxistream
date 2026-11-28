# Docker Compose

Runs Oxistream with PostgreSQL 18. The app listens on `127.0.0.1:8080` by default.

## Quickstart

```sh
cd deploy/compose
cp .env.example .env
# Set POSTGRES_PASSWORD (openssl rand -hex 32) and
# OXISTREAM_PROXY_SECRET (openssl rand -base64 48) in .env.
docker compose up -d
```

Without a prebuilt image, the first `up` builds it from the repository. To use a published image, set `OXISTREAM_IMAGE`, e.g. `ghcr.io/OWNER/oxistream:X.Y.Z`. Open `http://localhost:8080`; the first account becomes admin.

`.env` holds every setting, with defaults in comments. `OXISTREAM_BIND`, `OXISTREAM_BOTGUARD_BIN` and `OXISTREAM_YOUTUBE_CACHE_DIR` are set by the image.

The app runs with a read-only root filesystem and no capabilities. The `cache` volume keeps RustyPipe's client cache, the `postgres` volume the database.

## Upgrade

Migrations run at startup.

```sh
git pull && docker compose up -d --build   # built from source
docker compose pull && docker compose up -d   # published image
```

Major PostgreSQL upgrades need a dump and restore (or `pg_upgrade`); minor ones only a new tag.

## Backups

```sh
docker compose exec -T postgres pg_dump -U oxistream -Fc oxistream > oxistream-$(date +%F).dump
```

Restore into an empty database with `docker compose exec -T postgres pg_restore -U oxistream -d oxistream --clean --if-exists < oxistream-….dump`.

## Reverse proxy

Terminate TLS in a reverse proxy and forward to `127.0.0.1:8080` (or, sharing a Docker network, to `oxistream:8080`). Set `OXISTREAM_PUBLISH=8080` only to expose the port on all interfaces.

- `OXISTREAM_PUBLIC_URL=https://oxistream.example.com`: needed for OIDC and when the proxy rewrites `Host`, and used for Atom feed links.
- Keep `OXISTREAM_COOKIE_SECURE=true` behind HTTPS. Set it to `false` only for plain HTTP on a host other than localhost.
- Set `OXISTREAM_TRUSTED_PROXY_HEADER` (e.g. `X-Forwarded-For`) so rate limiting sees client IPs instead of the proxy's. Only set it when every request comes through the proxy.
- Watch parties use a WebSocket at `/api/v0/rooms/{id}/ws`: forward `Upgrade` and `Connection` headers and allow long-lived connections.
- Do not buffer `/proxy/` responses; video is streamed with range requests.

Caddy:

```
oxistream.example.com {
	reverse_proxy 127.0.0.1:8080
}
```

nginx:

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection $connection_upgrade;
    proxy_buffering off;
    proxy_read_timeout 1h;
}
```

`$connection_upgrade` needs the usual `map $http_upgrade $connection_upgrade { default upgrade; '' close; }` in the `http` block.

## OIDC

Set `OXISTREAM_OIDC_ISSUER`, `OXISTREAM_OIDC_CLIENT_ID` (and `OXISTREAM_OIDC_CLIENT_SECRET` for confidential clients) plus `OXISTREAM_PUBLIC_URL`. Register these with the provider:

- Redirect URI: `{PUBLIC_URL}/api/v0/auth/oidc/callback`
- Post-logout redirect URI: `{PUBLIC_URL}/`
- Back-channel logout URL (optional): `{PUBLIC_URL}/api/v0/auth/oidc/backchannel-logout`, with sessions required so logout tokens carry `sid`

With `OXISTREAM_OIDC_ADMIN_GROUP` set, the admin role follows the groups claim on every login; the value must match exactly (Keycloak full-path groups look like `/admins`). `/readyz` fails until provider discovery succeeds.
