import { pickImage } from '#lib/api/mapping.js';
import type { VideoDetails } from '#lib/api/video.js';
import type { PlaybackPreferences } from '#lib/components/player/playback.js';
import type { PlayerSubtitle } from '#lib/components/player/player.svelte';
import { m } from '#lib/paraglide/messages.js';
import type { PlayerMedia } from '#lib/player/app-player.svelte.js';

export function videoPoster(details: VideoDetails): string | undefined {
	return pickImage(details.thumbnails, 1280);
}

export function videoSubtitles(details: VideoDetails): PlayerSubtitle[] {
	return details.subtitles.map((s) => ({
		id: s.id,
		lang: s.lang,
		url: s.url,
		label: s.auto_generated ? m.player_auto_generated({ label: s.label }) : s.label
	}));
}

/** What the app-wide player needs to play a video, as the watch page loads it. */
export function watchMedia(
	details: VideoDetails,
	options: { startTime: number; playback: PlaybackPreferences; href: string }
): PlayerMedia {
	return {
		videoId: details.id,
		title: details.title,
		channel: details.channel,
		poster: videoPoster(details),
		subtitles: videoSubtitles(details),
		storyboard: details.storyboard_url ?? undefined,
		...options
	};
}
