import { createQuery } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import { api } from './client';
import { ApiError, unwrap } from './mapping';
import type { components } from './schema';

export type VideoDetails = components['schemas']['VideoDetails'];
export type Subtitle = components['schemas']['Subtitle'];

// Client errors will not change on a retry; upstream and rate-limit errors might.
function retry(failureCount: number, error: Error): boolean {
	if (error instanceof ApiError && error.status >= 400 && error.status < 500) return false;
	return failureCount < 2;
}

export function manifestUrl(id: string): string {
	return `/api/v0/videos/${encodeURIComponent(id)}/manifest.mpd`;
}

export async function fetchVideo(id: string, locale = getLocale()): Promise<VideoDetails> {
	return unwrap(
		await api.GET('/api/v0/videos/{id}', {
			params: { path: { id }, header: { 'Accept-Language': locale } }
		})
	);
}

/**
 * Loads the manifest once so the page can show why a video will not play. The player requests it
 * again; the backend marks it cacheable, so the browser answers that from its cache.
 */
export async function fetchManifest(id: string): Promise<string> {
	return unwrap<string>(
		await api.GET('/api/v0/videos/{id}/manifest.mpd', {
			params: { path: { id } },
			parseAs: 'text'
		})
	);
}

export function useVideo(id: () => string | null) {
	return createQuery(() => {
		const videoId = id();
		const locale = getLocale();
		return {
			queryKey: ['video', videoId, locale],
			queryFn: () => fetchVideo(videoId!, locale),
			enabled: videoId !== null,
			retry
		};
	});
}

export function useManifest(id: () => string | null) {
	return createQuery(() => {
		const videoId = id();
		return {
			queryKey: ['manifest', videoId],
			queryFn: () => fetchManifest(videoId!),
			enabled: videoId !== null,
			// Stream URLs in the manifest are signed for hours; never refetch on our own.
			staleTime: Infinity,
			retry
		};
	});
}
