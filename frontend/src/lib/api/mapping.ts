import type { ChannelSummary, VideoSummary } from '#lib/video.js';
import type { components } from './schema';

type Schemas = components['schemas'];
export type Thumbnail = Schemas['Thumbnail'];
export type ErrorBody = Schemas['ErrorBody'];

/** A failed API call, carrying the HTTP status and the backend's error code. */
export class ApiError extends Error {
	readonly status: number;
	readonly code?: ErrorBody['code'];
	readonly reason?: string | null;

	constructor(status: number, body?: Partial<ErrorBody>) {
		super(body?.message ?? `Request failed with status ${status}`);
		this.name = 'ApiError';
		this.status = status;
		this.code = body?.code;
		this.reason = body?.reason;
	}
}

/** The resource does not exist, or its id is malformed and so cannot exist. */
export function isNotFound(error: unknown): boolean {
	return error instanceof ApiError && (error.status === 404 || error.status === 400);
}

/** Unwraps an openapi-fetch result or throws an {@link ApiError}. */
export function unwrap<T>(result: { data?: T; error?: unknown; response: Response }): T {
	if (result.data !== undefined && result.response.ok) return result.data;
	const body =
		result.error && typeof result.error === 'object'
			? (result.error as Partial<ErrorBody>)
			: undefined;
	throw new ApiError(result.response.status, body);
}

/** The smallest image at least `width` pixels wide, else the largest one. */
export function pickImage(images: Thumbnail[] | undefined, width: number): string | undefined {
	if (!images?.length) return undefined;
	const sorted = [...images].sort((a, b) => a.width - b.width);
	return (sorted.find((image) => image.width >= width) ?? sorted[sorted.length - 1]).url;
}

export function toChannel(
	channel: Pick<
		Schemas['ChannelRef'],
		'id' | 'name' | 'avatar' | 'verified' | 'subscriber_count'
	> & {
		handle?: string | null;
	}
): ChannelSummary {
	return {
		id: channel.id,
		name: channel.name,
		handle: channel.handle ?? undefined,
		avatarUrl: pickImage(channel.avatar, 88),
		verified: channel.verified,
		subscriberCount: channel.subscriber_count ?? undefined
	};
}

/** Maps an API video; `fallbackChannel` fills in channel listings, which omit the channel. */
export function toVideo(
	video: Schemas['VideoSummary'],
	fallbackChannel?: ChannelSummary
): VideoSummary {
	return {
		id: video.id,
		title: video.title,
		channel: video.channel ? toChannel(video.channel) : (fallbackChannel ?? { id: '', name: '' }),
		durationSeconds: video.is_live ? undefined : (video.duration_seconds ?? undefined),
		viewCount: video.view_count ?? undefined,
		publishedAt: video.published_at ?? undefined,
		publishedText: video.published_text ?? undefined,
		isLive: video.is_live,
		description: video.description ?? undefined,
		thumbnailUrl: pickImage(video.thumbnails, 480)
	};
}

/** Drops repeated items; YouTube now and then returns an item again on a later page. */
export function uniqueBy<T>(items: T[], key: (item: T) => string): T[] {
	const seen = new Set<string>();
	return items.filter((item) => {
		const k = key(item);
		if (seen.has(k)) return false;
		seen.add(k);
		return true;
	});
}
