# Watch parties

Watch videos in sync with others. Only signed-in users can start a party, but anyone with the link can join.

## Start and invite

1. Start a party with "Watch together" on a watch page, or "Start a party" in the party panel (header button). The panel also lists your open parties.
2. "Invite" offers "Copy link", a QR code and the device's share dialog. The link looks like `/party/{id}`.
3. Guests enter a name to join, which is remembered in the browser. They can also sign in first.

Joining opens the watch page of the current video. Everyone can then browse the app while the party keeps playing in the mini player. If someone opens another video, the watch page offers "Play now" and "Add to queue" instead of its own player.

!!! info "Instances that require sign-in"

    Guests can still join by invite link and use the app while they are in the party. See [party guests](../advanced/authentication.md#party-guests).

## Party panel

=== "Queue"

    Add by search or link, "From library" (continue watching, liked videos, your playlists, "Queue all"), "Shuffle", "Autoplay", "Up next" with "Play now", reorder and remove.

    When the queue runs out with "Autoplay" on, the party plays a related video it has not played before. These show as "Added by autoplay".

=== "Chat"

    Text messages, up to 500 characters each.

    Reactions (heart, laugh, wow, sad, applause, fire) float over everyone's video.

=== "History"

    Videos played so far. Click one to play it again for everyone, or queue it again from its menu.

=== "Members"

    Host and guest labels, sync status ("In sync", "Buffering", "Out of sync"), place ("Watching", "Browsing", "On shorts"), and "Resync" for yourself.

## Host

The person who started the party is the host. Only the host can:

- remove participants. Removed people cannot rejoin.
- set "Control playback" and "Add videos" to "Everyone" (default) or "Host only".
- "End party": everyone is disconnected and the invite link stops working.

Everyone else can "Leave party" and come back with the link.

With "Control playback" set to "Everyone", anyone can play, pause, seek, skip, pick, reorder and shuffle.

## What changes in a party

- Party playback is not recorded in watch history.
- Loop, the sleep timer and the personal queue are off.
- SponsorBlock segments only show the skip button, and a skip moves everyone.

## Limits

| Limit | Value |
| --- | --- |
| People per party | 50 |
| Queued videos | 100 |
| Open parties per user | 10 |
| Lifetime without activity | 24 hours |
