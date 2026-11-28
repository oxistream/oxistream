# Import and export

Subscriptions are imported and exported at `/subscriptions/import-export` (signed in), also reachable from `/feed/channels`.

## Import

Import accepts files up to 5 MB:

| Source | File |
| --- | --- |
| YouTube (Google Takeout) | `subscriptions.csv`, any language |
| NewPipe | subscriptions JSON |
| Invidious | JSON data export, or OPML |
| FreeTube | OPML |
| Piped | subscriptions JSON |

The format is detected automatically. Afterwards you see how many channels were imported, already subscribed, skipped and invalid.

!!! note "Channel IDs only"

    Only entries with a channel ID (`/channel/UC…`) can be imported. Handles and other services are skipped and listed.

## Export

- NewPipe JSON, which also imports into Piped, LibreTube and Invidious.
- Invidious JSON.

Subscription groups are not part of the import or export.

## Playlists

YouTube playlists are copied one at a time: open the playlist and use "Save as copy", see [YouTube playlists](library.md#youtube-playlists).
