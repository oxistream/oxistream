import {
	createInfiniteQuery,
	createQuery,
	useQueryClient,
	type QueryClient
} from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import type { ChannelSummary, VideoSummary } from '#lib/video.js';
import { api } from './client';
import { ApiError, pickImage, toChannel, toVideo, unwrap } from './mapping';
import type { components } from './schema';

type VideoPage = components['schemas']['VideoPage'];

export type ChannelDetails = ChannelSummary & {
	description: string;
	bannerUrl?: string;
	/** Large avatar for the page header; `avatarUrl` stays small for chips. */
	avatarLargeUrl?: string;
	videoCount?: number;
	firstPage: VideoPage;
};

export type ChannelVideoPage = { items: VideoSummary[]; continuation?: string };

function channelOptions(id: string, locale: string) {
	return {
		queryKey: ['channel', locale, id] as const,
		queryFn: async ({ signal }: { signal: AbortSignal }): Promise<ChannelDetails> => {
			const channel = unwrap(
				await api.GET('/api/v0/channels/{id}', {
					params: { path: { id }, header: { 'Accept-Language': locale } },
					signal
				})
			);
			return {
				...toChannel(channel),
				description: channel.description,
				bannerUrl: pickImage(channel.banner, 1707),
				avatarLargeUrl: pickImage(channel.avatar, 160),
				videoCount: channel.video_count ?? undefined,
				firstPage: channel.videos
			};
		},
		staleTime: 5 * 60_000,
		// A missing or invalid channel will not appear on retry.
		retry: (count: number, error: Error) =>
			!(error instanceof ApiError && error.status < 500) && count < 2
	};
}

export function createChannelQuery(id: () => string | undefined) {
	return createQuery(() => {
		const channelId = id() ?? '';
		return { ...channelOptions(channelId, getLocale()), enabled: channelId.length > 0 };
	});
}

async function fetchChannelVideos(
	client: QueryClient,
	id: string,
	locale: string,
	continuation: string | undefined,
	signal: AbortSignal
): Promise<ChannelVideoPage> {
	// The channel response already carries the first page, so only later pages are fetched here.
	const details = await client.ensureQueryData(channelOptions(id, locale));
	const page = continuation
		? unwrap(
				await api.GET('/api/v0/channels/{id}/videos', {
					params: {
						path: { id },
						query: { continuation },
						header: { 'Accept-Language': locale }
					},
					signal
				})
			)
		: details.firstPage;
	const owner: ChannelSummary = {
		id: details.id,
		name: details.name,
		handle: details.handle,
		avatarUrl: details.avatarUrl,
		verified: details.verified
	};
	return {
		items: page.items.map((video) => toVideo(video, owner)),
		continuation: page.continuation ?? undefined
	};
}

export function createChannelVideosQuery(id: () => string | undefined) {
	const client = useQueryClient();
	return createInfiniteQuery(() => {
		const channelId = id() ?? '';
		const locale = getLocale();
		return {
			queryKey: ['channel-videos', locale, channelId] as const,
			queryFn: ({ pageParam, signal }: { pageParam: string | undefined; signal: AbortSignal }) =>
				fetchChannelVideos(client, channelId, locale, pageParam, signal),
			initialPageParam: undefined as string | undefined,
			getNextPageParam: (last: ChannelVideoPage) => last.continuation,
			enabled: channelId.length > 0,
			staleTime: 5 * 60_000
		};
	});
}

/** Looks up the channel id behind a "@handle" through the backend's link resolver. */
export function createHandleQuery(handle: () => string) {
	return createQuery(() => {
		const value = handle();
		return {
			queryKey: ['channel-handle', value.toLowerCase()] as const,
			queryFn: async ({ signal }: { signal: AbortSignal }) => {
				const target = unwrap(
					await api.GET('/api/v0/resolve', {
						params: { query: { url: `https://www.youtube.com/@${value}` } },
						signal
					})
				);
				if (target.kind !== 'channel') throw new ApiError(404);
				return target.id;
			},
			staleTime: 60 * 60_000,
			retry: (count: number, error: Error) =>
				!(error instanceof ApiError && error.status < 500) && count < 2
		};
	});
}
