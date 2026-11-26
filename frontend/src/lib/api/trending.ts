import { createQueries, createQuery } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import type { VideoSummary } from '#lib/video.js';
import { api } from './client';
import { toVideo, unwrap } from './mapping';
import type { components } from './schema';

export type TrendingCategory = components['schemas']['TrendingCategory'];

const TRENDING_STALE_MS = 15 * 60_000;

function trendingOptions(category: TrendingCategory, enabled: boolean) {
	const locale = getLocale();
	return {
		queryKey: ['trending', category, locale] as const,
		queryFn: async ({ signal }: { signal: AbortSignal }): Promise<VideoSummary[]> =>
			unwrap(
				await api.GET('/api/v0/trending', {
					params: { query: { category }, header: { 'Accept-Language': locale } },
					signal
				})
			).items.map((video) => toVideo(video)),
		enabled,
		staleTime: TRENDING_STALE_MS,
		// A failing category is hidden, so give up quickly instead of keeping its skeleton up.
		retry: 1
	};
}

/** Trending videos of the instance's region in one category. */
export function createTrendingQuery(category: () => TrendingCategory) {
	return createQuery(() => trendingOptions(category(), true));
}

/** Whether any of the categories is still loading, and how many of them have videos. */
export function createTrendingStatus(categories: () => TrendingCategory[], enabled: () => boolean) {
	return createQueries(() => ({
		queries: categories().map((category) => trendingOptions(category, enabled())),
		combine: (results) => ({
			pending: results.some((r) => r.isPending),
			withVideos: results.filter((r) => !!r.data?.length).length
		})
	}));
}
