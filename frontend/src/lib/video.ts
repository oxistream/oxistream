// UI-facing shapes for lists of videos. They will be mapped from the generated API types once the
// backend endpoints exist.

export type ChannelSummary = {
	id: string;
	name: string;
	handle?: string;
	avatarUrl?: string;
};

export type VideoSummary = {
	id: string;
	title: string;
	channel: ChannelSummary;
	durationSeconds: number;
	viewCount: number;
	publishedAt: string;
	thumbnailUrl?: string;
	/** Watched share between 0 and 1, when the user has started the video. */
	progress?: number;
};

export type User = {
	id: string;
	name: string;
	username: string;
	avatarUrl?: string;
};

export function initials(name: string): string {
	const parts = name
		.replace(/[^\p{L}\p{N}\s]/gu, ' ')
		.trim()
		.split(/\s+/)
		.filter(Boolean);
	if (parts.length === 0) return '?';
	const first = parts[0].charAt(0);
	const last = parts.length > 1 ? parts[parts.length - 1].charAt(0) : '';
	return (first + last).toUpperCase();
}
