# Docker Compose

[`deploy/compose/`](https://github.com/oxistream/oxistream/tree/main/deploy/compose) runs Oxistream with PostgreSQL 18. By default the app listens on `127.0.0.1:8080` only.

## Quickstart

```sh
cd deploy/compose
cp .env.example .env
# In .env, set POSTGRES_PASSWORD (openssl rand -hex 32)
# and OXISTREAM_PROXY_SECRET (openssl rand -base64 48).
docker compose up -d
```

If the image does not exist locally, the first `up` builds it from the repository. To use a published image, set `OXISTREAM_IMAGE=ghcr.io/OWNER/oxistream:X.Y.Z` in `.env`.

Open `http://localhost:8080`. The first local account to register becomes admin.

## Settings

[`.env.example`](https://github.com/oxistream/oxistream/blob/main/deploy/compose/.env.example) lists every setting, with its default in a comment. Each variable is described in [Configuration](../configuration.md). Compose builds `OXISTREAM_DATABASE_URL` from `POSTGRES_PASSWORD`. The image already sets `OXISTREAM_BIND`, `OXISTREAM_BOTGUARD_BIN` and `OXISTREAM_YOUTUBE_CACHE_DIR`.

Two variables only affect Compose:

| Variable | Default | Description |
| --- | --- | --- |
| `OXISTREAM_IMAGE` | `oxistream:local` | Image to run. Built from the repository if it does not exist. |
| `OXISTREAM_PUBLISH` | `127.0.0.1:8080` | Host binding of port 8080. `8080` exposes it on all interfaces. |

!!! note "Plain HTTP"

    Over plain HTTP on a host other than `localhost`, sign-in needs `OXISTREAM_COOKIE_SECURE=false`. For anything else, put a TLS-terminating [reverse proxy](../advanced/reverse-proxy.md) in front.
 A reverse proxy on the same Docker network reaches the app as `oxistream:8080`.

## Hardening

Both containers run with `no-new-privileges` and all capabilities dropped. The app's root filesystem is read-only, and `/tmp` is a tmpfs. The PostgreSQL container gets back the few capabilities its entrypoint needs to fix data directory ownership.

| Volume | Contents |
| --- | --- |
| `postgres` | The database. Back it up, see below. |
| `cache` | RustyPipe client cache. Safe to delete. |

## Upgrade

Migrations run at startup.

```sh
git pull && docker compose up -d --build        # image built from source
docker compose pull && docker compose up -d     # published image
```

A new PostgreSQL minor version only needs a new tag. A major upgrade needs a dump and restore, or `pg_upgrade`.

## Backups

```sh
docker compose exec -T postgres pg_dump -U oxistream -Fc oxistream > oxistream-$(date +%F).dump
```

To restore into an empty database:

```sh
docker compose exec -T postgres pg_restore -U oxistream -d oxistream --clean --if-exists < oxistream-YYYY-MM-DD.dump
```

See [Backups](../administration/upgrades.md#backups) for what the database contains.
