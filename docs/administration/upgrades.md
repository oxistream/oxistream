# Upgrades and backups

## Upgrades

1. Read the release notes. Before 1.0, the API (`/api/v0`) and behaviour may change between minor versions.
2. Back up the database (see below).
3. Deploy the new image. Migrations from `backend/migrations` run at startup, before the server listens. With several replicas, the first one to start applies them. The others wait on sqlx's migration lock.

!!! warning "No downgrades"

    Migrations only go forward. To go back, restore the backup taken before the upgrade.

Installation-specific steps are in [Docker](../setup/docker.md#upgrade), [Compose](../setup/compose.md#upgrade) and [Kubernetes](../setup/kubernetes.md).

## Backups

All state lives in PostgreSQL. That covers users, sessions, settings, subscriptions, groups, playlists, likes, history, blocked channels and hidden videos, watch parties and the video metadata cache.

`/var/lib/oxistream` only holds RustyPipe's cache and does not need a backup.

```sh
pg_dump -Fc -d "$OXISTREAM_DATABASE_URL" > oxistream-$(date +%F).dump
pg_restore --clean --if-exists -d "$OXISTREAM_DATABASE_URL" oxistream-YYYY-MM-DD.dump
```

Restore into the same or a newer PostgreSQL major version, with the server stopped. For Compose, see [Compose backups](../setup/compose.md#backups). On CloudNativePG, use its backup support (Barman or volume snapshots).

