import { createQuery } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import { api } from './client';
import type { components } from './schema';

export type VideoDetails = components['schemas']['VideoDetails'];
export type ApiVideoSummary = components['schemas']['VideoSummary'];
export type Thumbnail = components['schemas']['Thumbnail'];
export type ErrorBody = components['schemas']['ErrorBody'];
export type Subtitle = components['schemas']['Subtitle'];

/** A failed API call, carrying the backend's `{code, message, reason}` body when there was one. */
export class ApiError extends Error {
	readonly status: number;
	readonly code: ErrorBody['code'] | 'network';
	readonly reason: string | null;

	constructor(status: number, body?: Partial<ErrorBody>) {
		super(body?.message ?? `request failed with status ${status}`);
		this.name = 'ApiError';
		this.status = status;
		this.code = body?.code ?? (status === 0 ? 'network' : 'upstream_error');
		this.reason = body?.reason ?? null;
	}
}

function isErrorBody(value: unknown): value is ErrorBody {
	return typeof value === 'object' && value !== null && 'code' in value && 'message' in value;
}

// Client errors will not change on a retry; upstream and rate-limit errors might.
function retry(failureCount: number, error: Error): boolean {
	if (error instanceof ApiError && error.status >= 400 && error.status < 500) return false;
	return failureCount < 2;
}

/** The smallest thumbnail at least `minWidth` wide, else the largest one. */
export function pickThumbnail(thumbnails: Thumbnail[], minWidth: number): string | undefined {
	if (thumbnails.length === 0) return undefined;
	const sorted = [...thumbnails].sort((a, b) => a.width - b.width);
	return (sorted.find((t) => t.width >= minWidth) ?? sorted[sorted.length - 1]).url;
}

export function manifestUrl(id: string): string {
	return `/api/v0/videos/${encodeURIComponent(id)}/manifest.mpd`;
}

export async function fetchVideo(id: string, locale = getLocale()): Promise<VideoDetails> {
	const { data, error, response } = await api.GET('/api/v0/videos/{id}', {
		params: { path: { id }, header: { 'Accept-Language': locale } }
	});
	if (data) return data;
	throw new ApiError(response.status, isErrorBody(error) ? error : undefined);
}

/**
 * Loads the manifest once so the page can show why a video will not play. The player requests it
 * again; the backend marks it cacheable, so the browser answers that from its cache.
 */
export async function fetchManifest(id: string): Promise<string> {
	const { data, error, response } = await api.GET('/api/v0/videos/{id}/manifest.mpd', {
		params: { path: { id } },
		parseAs: 'text'
	});
	if (response.ok && typeof data === 'string') return data;
	let body: unknown = error;
	if (typeof error === 'string') {
		try {
			body = JSON.parse(error);
		} catch {
			body = undefined;
		}
	}
	throw new ApiError(response.status, isErrorBody(body) ? body : undefined);
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

export function useManifest(id: () => string | null, enabled: () => boolean = () => true) {
	return createQuery(() => {
		const videoId = id();
		return {
			queryKey: ['manifest', videoId],
			queryFn: () => fetchManifest(videoId!),
			enabled: videoId !== null && enabled(),
			// Stream URLs in the manifest are signed for hours; never refetch on our own.
			staleTime: Infinity,
			retry
		};
	});
}
