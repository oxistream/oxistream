import {
	createInfiniteQuery,
	createQuery,
	useQueryClient,
	type QueryClient
} from '@tanstack/svelte-query';
import { apiTabSegment, type ChannelSort, type VideoTab } from '#lib/channel-tabs.js';
import { getLocale } from '#lib/paraglide/runtime.js';
import type { ChannelSummary, VideoSummary } from '#lib/video.js';
import { api } from './client';
import { ApiError, pickImage, toChannel, toShort, toVideo, unwrap } from './mapping';
import type { PlaylistResult } from './search';
import type { components } from './schema';

type VideoPage = components['schemas']['VideoPage'];
type ApiVideo = components['schemas']['VideoSummary'];
type ApiPlaylist = components['schemas']['PlaylistSummary'];

export type ChannelDetails = ChannelSummary & {
	description: string;
	bannerUrl?: string;
	/** Large avatar for the page header; `avatarUrl` stays small for chips. */
	avatarLargeUrl?: string;
	videoCount?: number;
	hasShorts: boolean;
	hasLive: boolean;
	firstPage: VideoPage;
};

export type ChannelVideoPage = { items: VideoSummary[]; continuation?: string };
export type ChannelPlaylistPage = { items: PlaylistResult[]; continuation?: string };

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
				hasShorts: channel.has_shorts,
				hasLive: channel.has_live,
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

function ownerOf(details: ChannelDetails): ChannelSummary {
	return {
		id: details.id,
		name: details.name,
		handle: details.handle,
		avatarUrl: details.avatarUrl,
		verified: details.verified
	};
}

/** Maps a channel listing video; shorts get their portrait thumbnail. */
export function toChannelVideo(
	video: ApiVideo,
	owner: ChannelSummary,
	tab: VideoTab
): VideoSummary {
	return tab === 'shorts' ? toShort(video, owner) : toVideo(video, owner);
}

export function toChannelPlaylist(playlist: ApiPlaylist): PlaylistResult {
	return {
		id: playlist.id,
		title: playlist.title,
		thumbnailUrl: pickImage(playlist.thumbnails, 480),
		channel: playlist.channel ? toChannel(playlist.channel) : undefined,
		videoCount: playlist.video_count ?? undefined
	};
}

async function fetchChannelTab(
	client: QueryClient,
	id: string,
	tab: VideoTab,
	sort: ChannelSort,
	locale: string,
	continuation: string | undefined,
	signal: AbortSignal
): Promise<ChannelVideoPage> {
	const details = await client.ensureQueryData(channelOptions(id, locale));
	// The channel response already carries the first page of the latest videos.
	const page =
		tab === 'videos' && sort === 'latest' && !continuation
			? details.firstPage
			: unwrap(
					await api.GET(`/api/v0/channels/{id}/${apiTabSegment(tab)}`, {
						params: {
							path: { id },
							query: continuation ? { continuation } : { sort },
							header: { 'Accept-Language': locale }
						},
						signal
					})
				);
	const owner = ownerOf(details);
	return {
		items: page.items.map((video) => toChannelVideo(video, owner, tab)),
		continuation: page.continuation ?? undefined
	};
}

const pageOptions = {
	initialPageParam: undefined as string | undefined,
	getNextPageParam: (last: { continuation?: string }) => last.continuation,
	staleTime: 5 * 60_000,
	retry: 1
};

/** Videos, shorts or livestreams of a channel, page by page. */
export function createChannelTabQuery(
	id: () => string | undefined,
	tab: () => VideoTab,
	sort: () => ChannelSort
) {
	const client = useQueryClient();
	return createInfiniteQuery(() => {
		const channelId = id() ?? '';
		const t = tab();
		const s = sort();
		const locale = getLocale();
		return {
			...pageOptions,
			queryKey: ['channel-tab', locale, channelId, t, s] as const,
			queryFn: ({ pageParam, signal }: { pageParam: string | undefined; signal: AbortSignal }) =>
				fetchChannelTab(client, channelId, t, s, locale, pageParam, signal),
			enabled: channelId.length > 0
		};
	});
}

export function createChannelPlaylistsQuery(id: () => string | undefined) {
	return createInfiniteQuery(() => {
		const channelId = id() ?? '';
		const locale = getLocale();
		return {
			...pageOptions,
			queryKey: ['channel-playlists', locale, channelId] as const,
			queryFn: async ({
				pageParam,
				signal
			}: {
				pageParam: string | undefined;
				signal: AbortSignal;
			}): Promise<ChannelPlaylistPage> => {
				const page = unwrap(
					await api.GET('/api/v0/channels/{id}/playlists', {
						params: {
							path: { id: channelId },
							query: { continuation: pageParam },
							header: { 'Accept-Language': locale }
						},
						signal
					})
				);
				return {
					items: page.items.map(toChannelPlaylist),
					continuation: page.continuation ?? undefined
				};
			},
			enabled: channelId.length > 0
		};
	});
}

/** Videos of a channel matching `query`. */
export function createChannelSearchQuery(id: () => string | undefined, query: () => string) {
	const client = useQueryClient();
	return createInfiniteQuery(() => {
		const channelId = id() ?? '';
		const q = query().trim();
		const locale = getLocale();
		return {
			...pageOptions,
			queryKey: ['channel-search', locale, channelId, q] as const,
			queryFn: async ({
				pageParam,
				signal
			}: {
				pageParam: string | undefined;
				signal: AbortSignal;
			}): Promise<ChannelVideoPage> => {
				const details = await client.ensureQueryData(channelOptions(channelId, locale));
				const page = unwrap(
					await api.GET('/api/v0/channels/{id}/search', {
						params: {
							path: { id: channelId },
							query: pageParam ? { continuation: pageParam } : { q },
							header: { 'Accept-Language': locale }
						},
						signal
					})
				);
				const owner = ownerOf(details);
				return {
					items: page.items.map((video) => toVideo(video, owner)),
					continuation: page.continuation ?? undefined
				};
			},
			enabled: channelId.length > 0 && q.length > 0
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
