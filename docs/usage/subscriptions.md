# Subscriptions and feed

Subscribing needs an account. New subscriptions are fetched right away. After that, each channel is checked every 30 minutes, or whatever interval the instance sets.

To bring subscriptions from another service, see [Import and export](import-export.md).

## Feed

`/feed/subscriptions` shows a Shorts row and a grid of videos, newest first, until "You're all caught up". Feed filters:

- "Hide watched" and "Hide shorts", saved to your account.
- "Channel", to show a single channel.
- Group tabs ("All", then your groups).

"Not interested" in a video's menu hides it from the feed, trending rows and continue watching. "Undo" brings it back.

## Channels and groups

`/feed/channels` lists your subscriptions. There you can unsubscribe, assign groups per channel ("Groups for …"), manage groups and import or export. The "Groups" dialog creates, renames, deletes and reorders groups, and turns "Show on home" on or off. You can have up to 50 groups.

The sidebar lists your channels: 7 at first, then "… more". A dot marks channels that uploaded since you last opened their page, and those channels are listed first. Opening the channel page clears the dot.

## Feed reader link

Settings > Account > "RSS feed" creates a private feed URL with your latest 50 subscription videos, in Atom format. Any feed reader can use it.

- The link is shown only once. Copy it right away.
- "Regenerate" creates a new link and revokes the old one. "Delete" revokes it.

!!! warning

    Anyone with the link can read the feed, so treat it like a password.
