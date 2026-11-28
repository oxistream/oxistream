# Library

## Mini player and up next

Playback continues when you leave the watch page. The video moves to a mini player in the bottom right corner (a bottom card on phones). The mini player has "Back to the video", play/pause, "Next video", "Show queue" and "Close player". It is muted while you watch shorts.

The video menu has "Play next" and "Add to queue". The "Up next" panel on the watch page and in the mini player lets you:

- reorder entries by dragging,
- use "Play now" and "Remove" on each entry,
- "Clear queue".

Signed-in users' queue is saved to the account, so it follows them across tabs and devices. Guests' queue is saved in the browser and shared by its tabs. While you are in a [watch party](watch-parties.md), "Add to party" replaces the queue actions.

## Playlists

`/feed/playlists` has "Your playlists" (including Liked videos) and "Followed on YouTube".

### Your playlists

Signed in only.

- "New playlist": title, description and visibility.
    - Private: only you can see it.
    - Unlisted: anyone with the link can watch it, even without an account. "Copy link" shares it.
- "Save" on the watch page adds or removes the video in any of your playlists.
- Reorder by dragging, or with "Move to top", "Move up", "Move down" and "Move to bottom".
- "Play all". On your own playlists: "Continue" (resume at the last played video) or "Play from start".
- "Delete playlist" also stops shared links from working.

Liked videos (`/playlist?list=LL`) lists the videos you liked.

### YouTube playlists

Any YouTube playlist opens at `/playlist?list=PL…`. Signed-in users can:

- "Follow": the playlist appears under "Followed on YouTube" with a count of new videos. It is refreshed like a subscription.
- "Save as copy": imports it as one of your own playlists.

Mixes (`RD…`) cannot be opened as playlists.

## History

Signed in only. Guests' progress is kept in the browser.

`/feed/history` lists watched videos by day. You can:

- search the history,
- remove single videos,
- "Pause history": while paused, watched videos and resume points are not saved,
- "Clear history": this also removes resume points.

!!! danger "Clearing history cannot be undone"

    "Clear history" deletes every entry and resume point of your account.

Under Settings > History, "Keep history" deletes older entries automatically: Forever, 1 year, 6 months, 3 months, 1 month, 2 weeks, 1 week or 1 day. The instance may cap the longest option.

The video menu has "Mark as watched" and "Mark as unwatched". This is not available for shorts and livestreams. Video cards show how much you watched.
