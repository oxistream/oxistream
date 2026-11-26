import { createInfiniteQuery, createQuery } from '@tanstack/svelte-query';
import { pickPortrait } from '#lib/channel-tabs.js';
import { getLocale } from '#lib/paraglide/runtime.js';
import { session } from '#lib/session.svelte.js';
import type { VideoSummary } from '#lib/video.js';
import { api } from './client';
import { unwrap } from './mapping';
import type { components } from './schema';

export type DiscoveryPage = { items: VideoSummary[]; continuation?: string };
export type LinkedVideo = components['schemas']['LinkedVideo'];

/** YouTube sends no title or channel name with discovered shorts; the viewer loads them. */
export function toDiscoveredShort(short: components['schemas']['DiscoveredShort']): VideoSummary {
	return {
		id: short.id,
		title: '',
		channel: { id: short.channel_id ?? '', name: '' },
		thumbnailUrl: pickPortrait(short.thumbnails)?.url
	};
}

async function fetchDiscoveryPage(
	continuation: string | undefined,
	locale: string,
	signal: AbortSignal
): Promise<DiscoveryPage> {
	const page = unwrap(
		await api.GET('/api/v0/shorts/discover', {
			params: { query: { continuation }, header: { 'Accept-Language': locale } },
			signal
		})
	);
	return {
		items: page.items.map(toDiscoveredShort),
		continuation: page.continuation ?? undefined
	};
}

/** Endless shorts picked for the user, or from trending for anonymous users. */
export function createShortsDiscoveryQuery(enabled: () => boolean) {
	return createInfiniteQuery(() => {
		const locale = getLocale();
		return {
			queryKey: ['shorts', 'discover', session.user?.id ?? null, locale],
			queryFn: ({ pageParam, signal }: { pageParam: string | undefined; signal: AbortSignal }) =>
				fetchDiscoveryPage(pageParam, locale, signal),
			initialPageParam: undefined as string | undefined,
			getNextPageParam: (last: DiscoveryPage) => last.continuation,
			enabled: enabled(),
			// A new visit gets new shorts; within one the loaded pages stay put.
			staleTime: Infinity,
			gcTime: 0
		};
	});
}

/** The full video a short links to, once the short is on screen. */
export function createShortDetailsQuery(id: () => string | null) {
	return createQuery(() => {
		const videoId = id();
		const locale = getLocale();
		return {
			queryKey: ['shorts', 'details', videoId, locale],
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/shorts/{id}', {
						params: { path: { id: videoId! }, header: { 'Accept-Language': locale } },
						signal
					})
				),
			enabled: videoId !== null,
			staleTime: 60 * 60_000,
			retry: false
		};
	});
}
