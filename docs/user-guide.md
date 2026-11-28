---
title: User guide
nav_order: 2
---

# User guide

Oxistream lets you watch YouTube without a Google account. All video, images and subtitles come through your Oxistream instance. Some features need an account on the instance, see [Guests and accounts](#guests-and-accounts).

- [Home](#home)
- [Search](#search)
- [Watching](#watching)
- [Mini player and up next](#mini-player-and-up-next)
- [Shorts](#shorts)
- [Channels](#channels)
- [Subscriptions and feed](#subscriptions-and-feed)
- [Playlists](#playlists)
- [History](#history)
- [Watch parties](#watch-parties)
- [Import and export](#import-and-export)
- [Settings](#settings)
- [Guests and accounts](#guests-and-accounts)
- [YouTube-compatible URLs](#youtube-compatible-urls)

## Home

The home page is made of rows. Each row has "Show all", scroll buttons, and a "More actions" menu on every video. Empty rows hide themselves.

| Row | Content |
| --- | --- |
| Continue watching | Videos you started but did not finish. "Remove" drops one from the row. |
| Latest from your subscriptions | Newest uploads of your subscriptions. |
| Shorts from your subscriptions | Newest shorts of your subscriptions. |
| Subscription groups | One row for each group with "Show on home" turned on. |
| Trending: News, Gaming, Podcasts, Music, Sports, Learning, Fashion & Beauty, Live | Popular videos in the instance's region, from YouTube's topic pages. |

Signed-in users see Continue watching and the two subscription rows by default. Under Settings > Home page you can show, hide and reorder rows, including any trending topic. "Reset to default" restores the default rows.

Guests see News, Gaming and Podcasts. If none of them loads, guests see a search prompt and a "Recently watched" row built from progress saved in the browser.

## Search

- Press `/` to focus the search box. Suggestions appear as you type.
- Your last 10 searches are kept in the browser. Remove one with "Remove from search history", or all with "Clear search history".
- Paste a YouTube link (youtube.com, youtu.be, youtube-nocookie.com) into the search box to open it directly.

Results have the tabs All, Videos, Shorts, Channels and Playlists. "Filters" offers:

| Filter | Options | Tabs |
| --- | --- | --- |
| Upload date | Any time, Last hour, Today, This week, This month, This year | All, Videos, Shorts |
| Duration | Any length, Under 4 minutes, 4–20 minutes, Over 20 minutes | All, Videos |
| Sort by | Relevance, Newest, Most views | All, Videos, Shorts |

Results load as you scroll. Livestreams are marked "Live" and cannot be played yet.

## Watching

### Player

- Controls: play, back and forward 10 s, volume, seek bar with chapter segments, captions, picture-in-picture, theater mode (wide screens only) and fullscreen.
- Seek bar previews show storyboard thumbnails.
- Click or tap the video to play or pause. Double-tap the left or right side to seek, the middle to toggle fullscreen.
- The settings menu has Quality, Audio, Speed, Captions, Sleep timer, Loop, SponsorBlock and Keyboard shortcuts.
- Media keys and the operating system's media controls work: play, pause, seek, previous and next. The title, channel and artwork are shown there too.
- Volume, mute, speed and the last caption choice are remembered in the browser.

### Quality, codec and audio

Settings > Playback:

| Setting | Options | Who |
| --- | --- | --- |
| Video codec | Automatic, or a custom order of AV1, VP9 and H.264. Codecs your device cannot play are skipped. | Signed in |
| Maximum quality | No limit, or 4320p down to 144p | Signed in |
| Start quality | Automatic, or 4320p down to 144p | Signed in |
| Default audio track | Original (default), interface language, or one of 20 languages | Everyone (guests: saved in the browser) |

Videos with several audio tracks start with the original track, or with your preferred language if the video has it. Auto-dubbed tracks are labelled "(auto-dubbed)" and only play when you pick them in the Audio menu.

### Captions

Pick a track in the Captions menu, or press `C`. Automatic captions are labelled "(auto-generated)". The last choice is remembered.

### Chapters and transcript

The panel next to the video has two tabs:

- Chapters: click a chapter to jump to it. The current chapter is highlighted.
- Transcript: choose the language, search the transcript with previous/next match, and turn on "Follow playback" to auto-scroll.

Timestamps in the description are clickable. YouTube links in the description open inside Oxistream.

### Below the video

- Subscribe button and like button (signed in). There are no dislikes.
- "Save" adds the video to a playlist (signed in).
- "Watch together" starts a watch party (signed in).
- Related videos are listed beside or below the video.

When a video ends, the up-next queue plays first, then the next video of the playlist you are in. Otherwise playback stops. Related videos do not autoplay.

### Loop and sleep timer

- Loop (settings menu): repeats the current video. It turns off when another video loads.
- Sleep timer: 15, 30, 45 or 60 minutes, or "End of video". Minutes count real time, even while paused. When the timer runs out, playback pauses.

Neither is available in watch parties.

### Keyboard shortcuts

Press `?` on the watch page for the overlay. Shortcuts are ignored while typing in a text field.

| Key | Action |
| --- | --- |
| `K` or `Space` | Play or pause |
| `J` / `L` | Back / forward 10 seconds |
| `←` / `→` | Back / forward 5 seconds |
| `0`–`9` | Jump to 0–90 % |
| `Home` / `End` | Go to start / end |
| `<` / `>` | Slower / faster |
| `Shift+P` / `Shift+N` | Previous / next video |
| `↑` / `↓` | Volume up / down |
| `M` | Mute |
| `F` | Fullscreen |
| `T` | Theater mode |
| `C` | Captions |
| `I` | Picture-in-picture |
| `?` | Keyboard shortcuts |
| `/` | Focus search (anywhere) |

"Previous video" restarts the current video if it has played for more than 3 s. Otherwise it goes to the previous playlist video or the video you watched before.

### Resume

Videos resume where you left off. A `t=` in the link wins. For signed-in users, progress is saved to the account about every 10 seconds and on pause. Guests' progress stays in the browser. A video counts as watched from 90 %.

### SponsorBlock

Available when the instance has it enabled. Open it from Settings > SponsorBlock or the player menu.

| Category | Default |
| --- | --- |
| Sponsor | Skip |
| Self-promotion, Interaction reminder, Intro, Outro, Preview, Hook, Non-music, Filler | Ask |

- Skip: the segment is skipped automatically and a "Skipped: …" notice offers "Undo". A segment you undo or seek into is not skipped again.
- Ask: a "Skip …" button appears during the segment.
- Off: the segment is ignored.
- "Show the highlight" (on by default) marks the part most viewers look for and adds a "Jump to highlight" button.
- "Label promotional videos" (on by default) labels videos that are entirely about a product or service.

Segments are coloured on the seek bar, and hovering shows the category. Shorts only skip automatically. In watch parties, segments only show the skip button, and a skip moves everyone.

Guests' choices are saved in the browser.

### Videos that cannot play

Age-restricted, private, removed, region-blocked, members-only, Premium and paid videos show an explanation. Most error screens offer "Try again" and "Open on YouTube". Livestreams are not supported yet.

## Mini player and up next

Playback continues when you leave the watch page. The video moves to a mini player in the bottom right corner (a bottom card on phones). The mini player has "Back to the video", play/pause, "Next video", "Show queue" and "Close player". It is muted while you watch shorts.

The video menu has "Play next" and "Add to queue". The "Up next" panel on the watch page and in the mini player lets you:

- reorder entries by dragging,
- use "Play now" and "Remove" on each entry,
- "Clear queue".

Signed-in users' queue is saved to the account, so it follows them across tabs and devices. Guests' queue is saved in the browser and shared by its tabs. While you are in a watch party, "Add to party" replaces the queue actions.

## Shorts

`/shorts` is a vertical feed. Signed-in users can switch between "For you" (discovery) and "Subscriptions". Guests get discovery only. Shorts already shown in the browser session are skipped.

| Input | Action |
| --- | --- |
| `↓` or `J`, swipe up, scroll | Next short |
| `↑` or `K`, swipe down, scroll | Previous short |
| `Space`, `M`, `F`, `C`, `0`–`9` | Same as the video player |

The overlay shows the channel, subscribe and like buttons (signed in), and "Open in watch page". If the short links a full video, it shows that video too ("Full video: …").

## Channels

Channel pages (`/@handle` or `/channel/UC…`) show the banner, avatar, subscribers, video count and description. The tabs are:

- Videos, Shorts and Live: sort by Latest, Popular or Oldest.
- Playlists.
- Search this channel.

Signed-in users can subscribe, and block the channel from "More actions". Blocking a channel ends its subscription and leaves its videos out of search results, trending rows, related videos and shorts discovery. A blocked channel's page says so and offers "Unblock". Subscribing again lifts the block. Manage them under Settings > Blocked channels.

## Subscriptions and feed

Subscribing needs an account. New subscriptions are fetched right away. After that, each channel is checked every 30 minutes, or whatever interval the instance sets.

### Feed

`/feed/subscriptions` shows a Shorts row and a grid of videos, newest first, until "You're all caught up". Feed filters:

- "Hide watched" and "Hide shorts", saved to your account.
- "Channel", to show a single channel.
- Group tabs ("All", then your groups).

"Not interested" in a video's menu hides it from the feed, trending rows and continue watching. "Undo" brings it back.

### Channels and groups

`/feed/channels` lists your subscriptions. There you can unsubscribe, assign groups per channel ("Groups for …"), manage groups and import or export. The "Groups" dialog creates, renames, deletes and reorders groups, and turns "Show on home" on or off. You can have up to 50 groups.

The sidebar lists your channels: 7 at first, then "… more". A dot marks channels that uploaded since you last opened their page, and those channels are listed first. Opening the channel page clears the dot.

### Feed reader link

Settings > Account > "RSS feed" creates a private feed URL with your latest 50 subscription videos, in Atom format. Any feed reader can use it.

- The link is shown only once. Copy it right away.
- "Regenerate" creates a new link and revokes the old one. "Delete" revokes it.
- Anyone with the link can read the feed, so treat it like a password.

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
- "Clear history": this also removes resume points and cannot be undone.

Under Settings > History, "Keep history" deletes older entries automatically: Forever, 1 year, 6 months, 3 months, 1 month, 2 weeks, 1 week or 1 day. The instance may cap the longest option.

The video menu has "Mark as watched" and "Mark as unwatched". This is not available for shorts and livestreams. Video cards show how much you watched.

## Watch parties

Watch videos in sync with others. Only signed-in users can start a party, but anyone with the link can join.

### Start and invite

- Start a party with "Watch together" on a watch page, or "Start a party" in the party panel (header button). The panel also lists your open parties.
- "Invite" offers "Copy link", a QR code and the device's share dialog. The link looks like `/party/{id}`.
- Guests enter a name to join, which is remembered in the browser. They can also sign in first. If the instance requires login, guests cannot join.

Joining opens the watch page of the current video. Everyone can then browse the app while the party keeps playing in the mini player. If someone opens another video, the watch page offers "Play now" and "Add to queue" instead of its own player.

### Party panel

| Tab | Content |
| --- | --- |
| Queue | Add by search or link, "From library" (continue watching, liked videos, your playlists, "Queue all"), "Shuffle", "Autoplay", "Up next" with "Play now", reorder and remove. |
| Chat | Text messages, up to 500 characters each. |
| History | Videos played so far. |
| Members | Host and guest labels. Sync status ("In sync", "Buffering", "Out of sync"), place ("Watching", "Browsing", "On shorts"), and "Resync" for yourself. |

Reactions (heart, laugh, wow, sad, applause, fire) float over everyone's video.

When the queue runs out with "Autoplay" on, the party plays a related video it has not played before. These show as "Added by autoplay".

### Host

The person who started the party is the host. Only the host can:

- remove participants. Removed people cannot rejoin.
- set "Control playback" and "Add videos" to "Everyone" (default) or "Host only".
- "End party": everyone is disconnected and the invite link stops working.

Everyone else can "Leave party" and come back with the link.

With "Control playback" set to "Everyone", anyone can play, pause, seek, skip, pick, reorder and shuffle. Party playback is not recorded in watch history. Loop, the sleep timer and the personal queue are off in parties.

Parties end after 24 hours without activity. A party holds up to 50 people and 100 queued videos. Each user can have 10 open parties.

## Import and export

`/subscriptions/import-export` (signed in).

Import accepts files up to 5 MB:

| Source | File |
| --- | --- |
| YouTube (Google Takeout) | `subscriptions.csv`, any language |
| NewPipe | subscriptions JSON |
| Invidious | JSON data export, or OPML |
| FreeTube | OPML |
| Piped | subscriptions JSON |

The format is detected automatically. Afterwards you see how many channels were imported, already subscribed, skipped and invalid. Only entries with a channel ID (`/channel/UC…`) can be imported. Handles and other services are skipped and listed.

Export formats:

- NewPipe JSON, which also imports into Piped, LibreTube and Invidious.
- Invidious JSON.

## Settings

| Section | Contents | Who |
| --- | --- | --- |
| Playback | Codec, maximum and start quality, default audio track | Codec and quality signed in; audio everyone |
| Home page | Rows to show and their order | Signed in |
| History | Pause, retention, clear | Signed in |
| SponsorBlock | Per-category behaviour, highlight, promotional label | Everyone, if enabled on the instance |
| Blocked channels | Unblock | Signed in |
| Appearance | Theme (System, Light, Dark), language (English, Deutsch) | Everyone |
| Account | Account type and role, feed reader link, password change (local accounts), delete account | Signed in |

### Delete your account

Settings > Account > "Delete account". Type your username to confirm, plus your password if the account has one. This deletes your subscriptions, playlists, history, settings and sessions, and cannot be undone. The last admin of an instance cannot delete their account while other users exist. If you signed in with SSO, signing in again creates a new, empty account.

## Guests and accounts

| | Guest | Signed in |
| --- | --- | --- |
| Search, watch, shorts, channels, YouTube playlists | yes | yes |
| Open unlisted playlists by link | yes | yes |
| Join watch parties | yes, with a name (unless login is required) | yes |
| Resume points | in the browser | in the account |
| Up-next queue | in the browser | in the account |
| SponsorBlock settings, audio language | in the browser | SponsorBlock in the account, audio language in the browser |
| Subscriptions, feed, groups, import/export | – | yes |
| Likes, playlists, following YouTube playlists | – | yes |
| History page, mark as watched, not interested, block channels | – | yes |
| Start watch parties | – | yes |
| Home row customization, codec and quality settings | – | yes |

Instances can require sign-in for everything (`OXISTREAM_REQUIRE_LOGIN`), and can turn off registration.

## YouTube-compatible URLs

Oxistream uses YouTube's URL layout. Redirect extensions such as LibRedirect can send YouTube links to your instance by swapping the host.

| URL | Opens |
| --- | --- |
| `/watch?v={id}` | Video. Optional `t=` (or `start=`) start time, `list=` playlist. |
| `/{id}` | Video (youtu.be style). Redirects to `/watch?v={id}`, keeping `t=`. |
| `/shorts/{id}` | Short |
| `/@{handle}`, `/@{handle}/{tab}` | Channel. Tabs: `videos`, `shorts`, `streams`, `playlists`, `search`. |
| `/channel/UC…`, `/channel/UC…/{tab}` | Channel by ID |
| `/playlist?list={id}` | Playlist: YouTube, local, or `LL` for liked videos |
| `/results?search_query={query}` | Search, with optional `type`, `upload_date`, `duration`, `sort` |
| `/feed/subscriptions`, `/feed/history`, `/feed/playlists`, `/feed/channels` | Feed, history, playlists, subscribed channels |
| `/party/{id}` | Join a watch party |
