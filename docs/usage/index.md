# Usage

Oxistream lets you watch YouTube without a Google account. Video, images and subtitles come through your Oxistream instance. Some features need an account on the instance, see [guests and accounts](#guests-and-accounts).

<div class="grid cards" markdown>

-   :material-compass-outline:{ .lg .middle } **Browsing**

    ---

    The home page and its rows, search with filters, channel pages and the shorts feed.

    [:octicons-arrow-right-24: Browsing](browsing.md)

-   :material-play-box-outline:{ .lg .middle } **Watching**

    ---

    The player, quality and audio tracks, captions, chapters and transcript, resume, SponsorBlock.

    [:octicons-arrow-right-24: Watching](watching.md)

-   :material-rss:{ .lg .middle } **Subscriptions and feed**

    ---

    The subscription feed and its filters, channel groups, new-upload markers and the feed reader link.

    [:octicons-arrow-right-24: Subscriptions](subscriptions.md)

-   :material-bookshelf:{ .lg .middle } **Library**

    ---

    The mini player and up-next queue, your playlists, followed YouTube playlists and watch history.

    [:octicons-arrow-right-24: Library](library.md)

-   :material-account-group-outline:{ .lg .middle } **Watch parties**

    ---

    Watch in sync with others: invite links, the shared queue, chat, reactions and host settings.

    [:octicons-arrow-right-24: Watch parties](watch-parties.md)

-   :material-swap-horizontal:{ .lg .middle } **Import and export**

    ---

    Bring subscriptions from YouTube Takeout, NewPipe, Invidious, FreeTube or Piped, and take them along.

    [:octicons-arrow-right-24: Import and export](import-export.md)

-   :material-cog-outline:{ .lg .middle } **Settings**

    ---

    Playback, home page, history, SponsorBlock, appearance and your account.

    [:octicons-arrow-right-24: Settings](settings.md)

-   :material-keyboard-outline:{ .lg .middle } **Keyboard shortcuts**

    ---

    YouTube's keys in the player, plus a few of Oxistream's own.

    [:octicons-arrow-right-24: Shortcuts](shortcuts.md)

</div>

Links in YouTube's URL layout open in Oxistream as well, see [YouTube-compatible URLs](urls.md).

## Guests and accounts

| | Guest | Signed in |
| --- | --- | --- |
| Search, watch, shorts, channels, YouTube playlists | :material-check: | :material-check: |
| Open unlisted playlists by link | :material-check: | :material-check: |
| Join watch parties | :material-check: with a name | :material-check: |
| Resume points | in the browser | in the account |
| Up-next queue | in the browser | in the account |
| SponsorBlock settings, audio language | in the browser | SponsorBlock in the account, audio language in the browser |
| Subscriptions, feed, groups, import/export | – | :material-check: |
| Likes, playlists, following YouTube playlists | – | :material-check: |
| History page, mark as watched, not interested, block channels | – | :material-check: |
| Start watch parties | – | :material-check: |
| Home row customization, codec and quality settings | – | :material-check: |

!!! info "Instances that require sign-in"

    Instances can require sign-in for everything (`OXISTREAM_REQUIRE_LOGIN`), and can turn off registration. Guests can still join a watch party there by its invite link. While they are in the party, they can search and watch like other guests. Once they leave, are removed or the party ends, they have to sign in.
