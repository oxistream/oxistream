---
template: home.html
title: Self-hosted YouTube frontend
hide:
  - navigation
  - toc
---

<!-- Hero, feature grid and closing section live in overrides/home.html. -->

## Quick start with Docker Compose { #quick-start-compose }

Clone the repository, set two secrets and start the stack. The app listens on `127.0.0.1:8080`.

```sh
git clone https://github.com/oxistream/oxistream.git
cd oxistream/deploy/compose
cp .env.example .env
# In .env, set POSTGRES_PASSWORD (openssl rand -hex 32)
# and OXISTREAM_PROXY_SECRET (openssl rand -base64 48).
docker compose up -d
```

Open `http://localhost:8080` and register. The first local account becomes the admin. TLS, upgrades and backups are in [Docker Compose](setup/compose.md) and [Reverse proxy](advanced/reverse-proxy.md).
