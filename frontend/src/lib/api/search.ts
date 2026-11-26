import { createInfiniteQuery, createQuery, keepPreviousData } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import type { SearchType } from '#lib/search-types.js';
import type { ChannelSummary, VideoSummary } from '#lib/video.js';
import type { ResolvedTarget } from '#lib/youtube.js';
import { api } from './client';
import { pickImage, toChannel, toVideo, unwrap } from './mapping';
import type { components } from './schema';

type SearchItem = components['schemas']['SearchItem'];

export type ChannelResult = ChannelSummary & { description: string };

export type PlaylistResult = {
	id: string;
	title: string;
	thumbnailUrl?: string;
	channel?: ChannelSummary;
	videoCount?: number;
	isCourse?: boolean;
};

export type SearchResult =
	| { type: 'video'; key: string; video: VideoSummary }
	| { type: 'channel'; key: string; channel: ChannelResult }
	| { type: 'playlist'; key: string; playlist: PlaylistResult };

export type SearchPage = {
	items: SearchResult[];
	continuation?: string;
	correctedQuery?: string;
};

export function toSearchResult(item: SearchItem): SearchResult {
	switch (item.type) {
		case 'video':
			return { type: 'video', key: `v:${item.id}`, video: toVideo(item) };
		case 'channel':
			return {
				type: 'channel',
				key: `c:${item.id}`,
				channel: { ...toChannel(item), description: item.description }
			};
		case 'playlist':
			return {
				type: 'playlist',
				key: `p:${item.id}`,
				playlist: {
					id: item.id,
					title: item.title,
					thumbnailUrl: pickImage(item.thumbnails, 480),
					channel: item.channel ? toChannel(item.channel) : undefined,
					videoCount: item.video_count ?? undefined,
					isCourse: item.is_course
				}
			};
	}
}

async function fetchSearchPage(
	q: string,
	type: SearchType,
	locale: string,
	continuation: string | undefined,
	signal: AbortSignal
): Promise<SearchPage> {
	const page = unwrap(
		await api.GET('/api/v0/search', {
			params: {
				query: { q, continuation, type: type === 'all' ? undefined : type },
				header: { 'Accept-Language': locale }
			},
			signal
		})
	);
	return {
		items: page.items.map(toSearchResult),
		continuation: page.continuation ?? undefined,
		correctedQuery: page.corrected_query ?? undefined
	};
}

/** Paged search results; the next page is requested with the previous page's continuation. */
export function createSearchQuery(query: () => string, type: () => SearchType = () => 'all') {
	return createInfiniteQuery(() => {
		const q = query().trim();
		const kind = type();
		const locale = getLocale();
		return {
			queryKey: ['search', locale, kind, q] as const,
			queryFn: ({ pageParam, signal }: { pageParam: string | undefined; signal: AbortSignal }) =>
				fetchSearchPage(q, kind, locale, pageParam, signal),
			initialPageParam: undefined as string | undefined,
			getNextPageParam: (last: SearchPage) => last.continuation,
			enabled: q.length > 0,
			staleTime: 5 * 60_000,
			retry: 1
		};
	});
}

/** Search suggestions for the header box. Keeps the previous list while the next one loads. */
export function createSuggestionsQuery(query: () => string) {
	return createQuery(() => {
		const q = query().trim();
		const locale = getLocale();
		return {
			queryKey: ['search-suggestions', locale, q] as const,
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/search/suggestions', {
						params: { query: { q }, header: { 'Accept-Language': locale } },
						signal
					})
				),
			enabled: q.length > 0,
			placeholderData: keepPreviousData,
			staleTime: 10 * 60_000,
			retry: false
		};
	});
}

/** Resolves a pasted YouTube link to a video, channel or playlist. */
export async function resolveYouTubeUrl(
	url: string,
	signal?: AbortSignal
): Promise<ResolvedTarget> {
	return unwrap(await api.GET('/api/v0/resolve', { params: { query: { url } }, signal }));
}
