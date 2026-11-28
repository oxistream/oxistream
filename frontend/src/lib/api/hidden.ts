import type { QueryClient } from '@tanstack/svelte-query';
import { api } from './client';
import { historyKeys } from './history';
import { unwrap } from './mapping';
import { subscriptionKeys } from './subscriptions';
import { withoutVideo } from './video-cache';

/** Listings the backend leaves hidden videos out of. */
const HIDDEN_FROM = [subscriptionKeys.feed, ['trending'], historyKeys.continue] as const;

async function send(videoId: string, hide: boolean) {
	const params = { params: { path: { video_id: videoId } } };
	const { response, error } = hide
		? await api.PUT('/api/v0/hidden-videos/{video_id}', params)
		: await api.DELETE('/api/v0/hidden-videos/{video_id}', params);
	if (!response.ok) unwrap({ error, response });
}

function refetch(client: QueryClient) {
	for (const queryKey of HIDDEN_FROM) void client.invalidateQueries({ queryKey });
}

/**
 * Marks a video as not interesting, dropping it from loaded feeds and home rows right away.
 * Rejects when the server refuses; the lists are refetched either way.
 */
export async function hideVideo(client: QueryClient, videoId: string) {
	for (const queryKey of HIDDEN_FROM) {
		await client.cancelQueries({ queryKey });
		client.setQueriesData({ queryKey }, (data: unknown) =>
			data === undefined ? data : withoutVideo(data, videoId)
		);
	}
	try {
		await send(videoId, true);
	} finally {
		refetch(client);
	}
}

export async function unhideVideo(client: QueryClient, videoId: string) {
	try {
		await send(videoId, false);
	} finally {
		refetch(client);
	}
}
