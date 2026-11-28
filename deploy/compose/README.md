# Docker Compose

Runs Oxistream with PostgreSQL 18, on `127.0.0.1:8080` by default.

```sh
cp .env.example .env
# Set POSTGRES_PASSWORD (openssl rand -hex 32) and
# OXISTREAM_PROXY_SECRET (openssl rand -base64 48) in .env.
docker compose up -d
```

Setup, upgrades and backups: [docs/install/compose.md](../../docs/install/compose.md). Variables: [docs/configuration.md](../../docs/configuration.md). TLS and reverse proxies: [docs/reverse-proxy.md](../../docs/reverse-proxy.md).
