# Watching

## Player

- Controls: play, back and forward 10 s, volume, seek bar with chapter segments, captions, picture-in-picture, theater mode (wide screens only) and fullscreen.
- Seek bar previews show storyboard thumbnails.
- Click or tap the video to play or pause. Double-tap the left or right side to seek, the middle to toggle fullscreen.
- The settings menu has Quality, Audio, Speed, Captions, Sleep timer, Loop, SponsorBlock and Keyboard shortcuts.
- Media keys and the operating system's media controls work: play, pause, seek, previous and next. The title, channel and artwork are shown there too.
- Volume, mute, speed and the last caption choice are remembered in the browser.

All keys are listed in [Keyboard shortcuts](shortcuts.md).

## Quality, codec and audio

Settings > Playback:

| Setting | Options | Who |
| --- | --- | --- |
| Video codec | Automatic, or a custom order of AV1, VP9 and H.264. Codecs your device cannot play are skipped. | Signed in |
| Maximum quality | No limit, or 4320p down to 144p | Signed in |
| Start quality | Automatic, or 4320p down to 144p | Signed in |
| Default audio track | Original (default), interface language, or one of 20 languages | Everyone (guests: saved in the browser) |

Videos with several audio tracks start with the original track, or with your preferred language if the video has it. Auto-dubbed tracks are labelled "(auto-dubbed)" and only play when you pick them in the Audio menu.

## Captions

Pick a track in the Captions menu, or press ++c++. Automatic captions are labelled "(auto-generated)". The last choice is remembered.

## Chapters and transcript

The panel next to the video has two tabs:

- Chapters: click a chapter to jump to it. The current chapter is highlighted.
- Transcript: choose the language, search the transcript with previous/next match, and turn on "Follow playback" to auto-scroll.

Timestamps in the description are clickable. YouTube links in the description open inside Oxistream.

## Below the video

- Subscribe button and like button (signed in). There are no dislikes.
- "Save" adds the video to a playlist (signed in).
- "Watch together" starts a [watch party](watch-parties.md) (signed in).
- Related videos are listed beside or below the video.

When a video ends, the [up-next queue](library.md#mini-player-and-up-next) plays first, then the next video of the playlist you are in. Otherwise playback stops. Related videos do not autoplay.

## Loop and sleep timer

- Loop (settings menu): repeats the current video. It turns off when another video loads.
- Sleep timer: 15, 30, 45 or 60 minutes, or "End of video". Minutes count real time, even while paused. When the timer runs out, playback pauses.

Neither is available in watch parties.

## Resume

Videos resume where you left off. A `t=` in the link wins. For signed-in users, progress is saved to the account about every 10 seconds and on pause. Guests' progress stays in the browser. A video counts as watched from 90 %.

## SponsorBlock

Available when the instance has it enabled. Open it from Settings > SponsorBlock or the player menu.

| Category | Default |
| --- | --- |
| Sponsor | Skip |
| Self-promotion, Interaction reminder, Intro, Outro, Preview, Hook, Non-music, Filler | Ask |

Skip
:   The segment is skipped automatically and a "Skipped: …" notice offers "Undo". A segment you undo or seek into is not skipped again.

Ask
:   A "Skip …" button appears during the segment.

Off
:   The segment is ignored.

"Show the highlight" (on by default) marks the part most viewers look for and adds a "Jump to highlight" button. "Label promotional videos" (on by default) labels videos that are entirely about a product or service.

Segments are coloured on the seek bar, and hovering shows the category. Shorts only skip automatically. In watch parties, segments only show the skip button, and a skip moves everyone.

Guests' choices are saved in the browser. The server looks up the segments, see [Privacy](../advanced/privacy.md#what-reaches-sponsorblock).

## Videos that cannot play

Age-restricted, private, removed, region-blocked, members-only, Premium and paid videos show an explanation. Most error screens offer "Try again" and "Open on YouTube".

!!! note "Livestreams"

    Livestreams are not supported yet. Their watch page asks you to come back once the stream has ended.
