---
title: Privacy
nav_order: 9
---

# Privacy

This page describes what leaves the server, what reaches third parties, and what is stored. It reflects the current code.

## What the browser contacts

The browser talks only to the Oxistream instance: the SPA, `/api` and `/proxy`. All video, audio, thumbnails, avatars, banners, storyboards and subtitles are served through the instance's proxy. Fonts are bundled.

The browser only goes to youtube.com when the user follows an explicit link, such as "Open on YouTube".

## What reaches YouTube

The server talks to YouTube from its own IP, without accounts or cookies (RustyPipe in unauthenticated mode).

| Trigger | What YouTube receives |
| --- | --- |
| Search, search suggestions | The query text, as typed (suggestions). |
| Watch, shorts, channel, playlist pages | The video, channel or playlist ID. |
| Trending rows | Requests for the destination pages of `OXISTREAM_TRENDING_REGION`. |
| Shorts discovery | Seed video IDs only. For signed-in users, seeds come from their liked shorts and recent subscription shorts. |
| Subscription feed and followed playlists | The feed worker fetches each channel or playlist once per interval, however many users follow it. No user information is sent. |
| Playback | Range requests for stream bytes, thumbnail and subtitle requests. |

Proxy requests to YouTube carry RustyPipe's client User-Agent and the client's `Range` header. Client cookies, IP addresses and other headers are not forwarded. Upstream hosts are limited to an allowlist, and redirects are checked against it too:

- `*.googlevideo.com` (streams)
- `*.ytimg.com`, `*.ggpht.com`, `yt3.googleusercontent.com` (images)
- `*.youtube.com` (subtitles)

YouTube sees one client, the server. It can still connect requests made from that IP, e.g. a search followed by a watch.

## What reaches SponsorBlock

Only if `OXISTREAM_SPONSORBLOCK_ENABLED` is on (the default). The server asks `OXISTREAM_SPONSORBLOCK_API_URL` for `/api/skipSegments/{prefix}`, where the prefix is the first four hex characters of the SHA-256 of the video ID. It then picks out the requested video locally. Many videos share each prefix.

Requests carry the User-Agent `Oxistream/<version>` and no user data. Browsers never contact SponsorBlock.

## What reaches the identity provider

Only with OIDC. The usual authorization code flow, token and userinfo requests, and the end-session redirect on logout. The provider sends back-channel logout requests to the server.

## What is stored

Everything lives in PostgreSQL.

| Data | Contents | Removed when |
| --- | --- | --- |
| Account | Username, argon2id password hash (local accounts), role, disabled flag, creation time. For OIDC: issuer and subject. | The account is deleted |
| Sessions | SHA-256 of the token, created and last-seen time, expiry, user agent and IP at sign-in, OIDC `sid` and ID token | Logout, expiry, password change (other sessions), back-channel logout, account deletion |
| Settings | Playback preferences, home rows, feed filters, history pause and retention, SponsorBlock preferences | Account deletion |
| Subscriptions | Channels, groups, last visit per channel | Unsubscribe, account deletion |
| Playlists, likes, followed YouTube playlists | IDs, order, titles, descriptions | User action, account deletion |
| Watch history | Video, position, duration, finished, last watched | The retention setting (cleanup runs hourly), "Clear history", per-video removal, account deletion. Nothing is recorded while history is paused, or for party playback. |
| Up-next queue | Video IDs | User action, account deletion |
| Blocked channels, "not interested" videos | IDs | User action, account deletion |
| Atom feed token | SHA-256 only | Regenerate, delete, account deletion |
| Watch parties | Queue, played history, chat (latest 100 messages), participants with display names, SHA-256 of guest keys, removed participants | Room ends or expires (24 h idle) |
| Shared caches | Video and channel metadata, channel uploads, the first 500 items of followed YouTube playlists | Not tied to users |
| Back-channel logout | `jti` of used logout tokens | Hourly cleanup after the acceptance window |

Account deletion removes everything the account owns. An admin can also delete accounts, see [Administration](administration.md#users).

In memory, per replica (not persisted): player responses, trending pages, search suggestions, SponsorBlock segments, rate-limiter state.

### Logs

Request logs contain method, path with query, and status. Client IPs are not logged. Atom feed tokens in paths are replaced with `<redacted>`. Search queries appear in the logged query string of `/api/v0/search` requests. Configure log retention accordingly.

### In the browser

Guests keep their state in `localStorage`: resume progress, queue, SponsorBlock preferences, audio language, volume, captions and watch party name. Both guests and signed-in users keep recent searches, theme and language there. The current watch party and the shorts already shown are kept in `sessionStorage`. Clearing site data removes all of it.
