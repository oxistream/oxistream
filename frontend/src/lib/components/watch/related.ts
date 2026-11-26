import { toVideo, uniqueBy } from '#lib/api/mapping.js';
import type { components } from '#lib/api/schema.js';
import type { VideoSummary } from '#lib/video.js';

/** Livestreams, upcoming premieres and shorts do not play here, so they are left out. */
export function playableRelated(videos: components['schemas']['VideoSummary'][]): VideoSummary[] {
	return uniqueBy(
		videos.filter((v) => !v.is_live && !v.is_upcoming && !v.is_short),
		(v) => v.id
	).map((v) => toVideo(v));
}
