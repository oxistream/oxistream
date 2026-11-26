import { pickThumbnail, type ApiVideoSummary } from '#lib/api/video.js';

/** A related video. YouTube often leaves out the channel, views and date in this list. */
export type RelatedVideo = {
	id: string;
	title: string;
	thumbnailUrl?: string;
	durationSeconds?: number | null;
	viewCount?: number | null;
	publishedAt?: string | null;
	publishedText?: string | null;
	channel?: { id: string; name: string } | null;
};

export function toRelatedVideo(video: ApiVideoSummary): RelatedVideo {
	return {
		id: video.id,
		title: video.title,
		thumbnailUrl: pickThumbnail(video.thumbnails, 336),
		durationSeconds: video.duration_seconds,
		viewCount: video.view_count,
		publishedAt: video.published_at,
		publishedText: video.published_text,
		channel: video.channel ? { id: video.channel.id, name: video.channel.name } : null
	};
}

/** Livestreams, upcoming premieres and shorts do not play here, so they are left out. */
export function playableRelated(videos: ApiVideoSummary[]): RelatedVideo[] {
	const seen = new Set<string>();
	return videos
		.filter((v) => !v.is_live && !v.is_upcoming && !v.is_short && !seen.has(v.id) && seen.add(v.id))
		.map(toRelatedVideo);
}
