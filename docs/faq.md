# FAQ

## Do I need a Google or YouTube account?

No. The server talks to YouTube without accounts or cookies, and Oxistream never asks for one. Accounts exist only on your instance.

## Does my browser contact YouTube or Google?

No. Video, audio, thumbnails, avatars and subtitles come through the instance's proxy, and fonts are bundled. The browser only goes to youtube.com when you follow a link such as "Open on YouTube". Details are in [Privacy](advanced/privacy.md).

## Can I use it without an account?

Yes, unless the instance sets `OXISTREAM_REQUIRE_LOGIN`. Guests can search, watch, open channels, shorts and YouTube playlists, and join watch parties. Subscriptions, playlists and history need an account. See [guests and accounts](usage/index.md#guests-and-accounts).

## How do I move my subscriptions from YouTube, NewPipe, Invidious or Piped?

Import the export file of that service at `/subscriptions/import-export`. YouTube's comes from Google Takeout as `subscriptions.csv`. See [Import and export](usage/import-export.md).

## Can I watch livestreams?

Not yet. Livestreams show up in search and on channel pages, but their watch page asks you to come back once the stream has ended.

## Are comments shown?

Not yet.

## Does SponsorBlock see what I watch?

The server asks SponsorBlock for a 4-character hash prefix of the video ID, which many videos share, and picks the right video locally. Browsers never contact SponsorBlock, and the instance can turn it off with `OXISTREAM_SPONSORBLOCK_ENABLED=false`.

## Which databases are supported?

PostgreSQL 18 or newer. There is no SQLite support yet.

## Who becomes the admin?

The first local account registered on the instance. SSO accounts never become admin by being first: with `OXISTREAM_OIDC_ADMIN_GROUP`, their role follows that group of the identity provider (required for SSO-only instances), see [admin role from groups](advanced/authentication.md#admin-role-from-groups). Without an admin group, admins make other users admins in the [admin area](administration/index.md).

## A user forgot their password. Can an admin reset it?

No. There is no password reset, and admins cannot set passwords. An admin can delete the account, and the user registers again while registration is open; the old account's data is lost. Instances with SSO can leave passwords to the identity provider.

## Can I use more than one SSO provider?

No. Oxistream supports exactly one OpenID Connect provider, next to local accounts. An SSO identity is never linked to an existing local account, even when the names match.

## Can I run several replicas?

Yes. They need the same `OXISTREAM_PROXY_SECRET`, one egress IP towards YouTube, synced clocks and direct or session-mode database connections. See [Replicas and rate limiting](advanced/scaling.md).

## Why does YouTube block my instance?

YouTube throttles IPs that send many requests, datacenter IPs sooner than residential ones. See [YouTube blocks and 403s](troubleshooting.md#youtube-blocks-and-403s).

## Do redirect extensions such as LibRedirect work?

Yes. Oxistream uses YouTube's URL layout, so swapping the host is enough. See [YouTube-compatible URLs](usage/urls.md).

## Which languages does the interface support?

English and German. Pick one under Settings > Appearance.
