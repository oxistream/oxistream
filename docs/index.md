---
template: home.html
title: Self-hosted YouTube frontend
hide:
  - navigation
  - toc
---

<!-- The hero (name, pitch, screenshots, buttons) lives in overrides/home.html. -->

## What it does { #features }

<div class="grid cards" markdown>

- **Watching**

    DASH player with all qualities, codecs and audio tracks, subtitles, chapters, a searchable transcript, keyboard shortcuts, sleep timer and loop. A mini player keeps playing while you browse.

- **Discovery**

    Search, channel pages, a shorts player, and a home page with continue watching, subscriptions and trending rows. URLs mirror YouTube's, so redirect extensions work.

- **Subscriptions**

    A feed with filters (hide watched, hide shorts, per channel), subscription groups, new-upload markers and a private Atom feed. Import from YouTube Takeout, NewPipe, Invidious and Piped; export to NewPipe and Invidious.

- **Library**

    Local playlists, private or shared by link, followed YouTube playlists, an up-next queue synced across devices, and watch history with resume, search and retention.

- **Watch together**

    Parties with an invite link, a shared queue, synchronized playback, chat and reactions. Guests can join without an account.

- **SponsorBlock**

    Segment markers, and auto-skip or a skip button per category. The server looks up the segments.

- **Accounts**

    Local accounts and OpenID Connect with back-channel logout, an optional login requirement, account self-deletion and a small admin area.

- **Interface**

    Light and dark mode, in English and German.

- **Operations**

    One container image for amd64 and arm64 next to PostgreSQL, multiple replicas, Prometheus metrics, health probes, Docker Compose and a Helm chart.

</div>

## Quick start with Docker Compose { #quick-start }

```sh
git clone https://github.com/oxistream/oxistream.git
cd oxistream/deploy/compose
cp .env.example .env
# In .env, set POSTGRES_PASSWORD (openssl rand -hex 32)
# and OXISTREAM_PROXY_SECRET (openssl rand -base64 48).
docker compose up -d
```

Open `http://localhost:8080` and register. The first account becomes the admin. For TLS, upgrades and backups, read [Docker Compose](install/compose.md) and [Reverse proxy](reverse-proxy.md).
