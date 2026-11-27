import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { session } from '#lib/session.svelte.js';
import { api } from './client';
import { unwrap } from './mapping';
import type { components } from './schema';

export type FeedTokenStatus = components['schemas']['FeedTokenStatus'];
export type CreatedFeedToken = components['schemas']['CreatedFeedToken'];

const feedTokenKey = ['settings', 'feed-token'] as const;

/** Whether the signed-in user has an Atom feed URL; idle for guests. */
export function createFeedTokenQuery() {
	return createQuery(() => ({
		queryKey: feedTokenKey,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/settings/feed-token', { signal })),
		enabled: session.user !== null
	}));
}

/** Creates a feed URL, revoking any previous one. The URL is only returned here. */
export function createFeedTokenMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async () => unwrap(await api.POST('/api/v0/settings/feed-token')),
		onSuccess: (created: CreatedFeedToken) =>
			client.setQueryData<FeedTokenStatus>(feedTokenKey, { created_at: created.created_at })
	}));
}

export function createDeleteFeedTokenMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async () => {
			const { response, error } = await api.DELETE('/api/v0/settings/feed-token');
			if (!response.ok) unwrap({ error, response });
		},
		onSuccess: () => client.setQueryData<FeedTokenStatus>(feedTokenKey, { created_at: null })
	}));
}
