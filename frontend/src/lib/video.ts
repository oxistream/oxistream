// UI-facing shapes for lists of videos, mapped from the generated API types in `#lib/api/mapping.ts`.

export type ChannelSummary = {
	id: string;
	name: string;
	handle?: string;
	avatarUrl?: string;
	verified?: boolean;
	subscriberCount?: number;
};

export type VideoSummary = {
	id: string;
	title: string;
	channel: ChannelSummary;
	/** Missing for live streams and in channel listings that do not report it. */
	durationSeconds?: number;
	viewCount?: number;
	/** Current viewers of a live stream, which has no view count. */
	viewers?: number;
	publishedAt?: string;
	/** YouTube's own localized date text, used when no exact date is known. */
	publishedText?: string;
	isLive?: boolean;
	isShort?: boolean;
	/** A premiere or scheduled stream that cannot be played yet. */
	isUpcoming?: boolean;
	description?: string;
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
