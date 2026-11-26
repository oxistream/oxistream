import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import { session } from '#lib/session.svelte.js';
import type { ChannelSummary } from '#lib/video.js';
import { api } from './client';
import { pickImage, unwrap } from './mapping';
import type { components } from './schema';
import { subscriptionKeys } from './subscriptions';

export type BlockedChannel = {
	id: string;
	name: string;
	avatarUrl?: string;
	blockedAt: string;
};

export const blockKeys = {
	list: ['blocks'] as const
};

/** Listings the backend filters; their cached pages lose a channel as soon as it is blocked. */
const FILTERED = ['search', 'trending', 'video', 'feed'] as const;
/** Listings that change when a channel is blocked or unblocked, refetched afterwards. */
const AFFECTED = [...FILTERED, 'channel-playlists', 'channel-search'] as const;

export function toBlockedChannel(channel: components['schemas']['BlockedChannel']): BlockedChannel {
	return {
		id: channel.id,
		name: channel.name,
		avatarUrl: pickImage(channel.avatar, 88),
		blockedAt: channel.blocked_at
	};
}

function channelOf(item: unknown): string | undefined {
	if (!item || typeof item !== 'object') return undefined;
	const { channel, video, playlist } = item as {
		channel?: { id?: unknown };
		video?: { channel?: { id?: unknown } };
		playlist?: { channel?: { id?: unknown } };
	};
	const id = channel?.id ?? video?.channel?.id ?? playlist?.channel?.id;
	return typeof id === 'string' ? id : undefined;
}

/**
 * Drops every list entry of `channelId` from cached query data, however deep: videos, shorts,
 * playlists and the channel itself in search results. Objects outside lists are kept.
 */
export function withoutChannel<T>(data: T, channelId: string): T {
	if (Array.isArray(data)) {
		return (data as unknown[])
			.filter((item) => channelOf(item) !== channelId)
			.map((item) => withoutChannel(item, channelId)) as T;
	}
	if (data && typeof data === 'object') {
		return Object.fromEntries(
			Object.entries(data).map(([key, value]) => [key, withoutChannel(value, channelId)])
		) as T;
	}
	return data;
}

/** The signed-in user's blocked channels, most recent first; idle for anonymous users. */
export function createBlocksQuery() {
	return createQuery(() => ({
		queryKey: blockKeys.list,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/blocks', { signal })).map(toBlockedChannel),
		enabled: session.user !== null,
		staleTime: 5 * 60_000
	}));
}

export type BlockChange = {
	channel: Pick<ChannelSummary, 'id' | 'name' | 'avatarUrl'>;
	block: boolean;
};

/**
 * Blocks or unblocks a channel. Blocking hides it from loaded listings right away and ends the
 * subscription, as the backend does.
 */
export function createBlockMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ channel, block }: BlockChange) => {
			const path = { channel_id: channel.id };
			if (block) {
				unwrap(
					await api.PUT('/api/v0/blocks/{channel_id}', {
						params: { path, header: { 'Accept-Language': getLocale() } }
					})
				);
				return;
			}
			const { response, error } = await api.DELETE('/api/v0/blocks/{channel_id}', {
				params: { path }
			});
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async ({ channel, block }: BlockChange) => {
			await client.cancelQueries({ queryKey: blockKeys.list });
			const previous = client.getQueryData<BlockedChannel[]>(blockKeys.list);
			const others = (previous ?? []).filter((c) => c.id !== channel.id);
			client.setQueryData<BlockedChannel[]>(
				blockKeys.list,
				block
					? [
							{
								id: channel.id,
								name: channel.name,
								avatarUrl: channel.avatarUrl,
								blockedAt: new Date().toISOString()
							},
							...others
						]
					: others
			);
			if (block) {
				client.setQueryData(subscriptionKeys.state(channel.id), false);
				for (const key of FILTERED) {
					client.setQueriesData({ queryKey: [key] }, (data: unknown) =>
						data === undefined ? data : withoutChannel(data, channel.id)
					);
				}
			}
			return { previous };
		},
		onError: (_error: Error, _change: BlockChange, context?: { previous?: BlockedChannel[] }) => {
			client.setQueryData(blockKeys.list, context?.previous);
		},
		// Not awaited, so the button settles without waiting for the refetch.
		onSettled: () => {
			void client.invalidateQueries({ queryKey: blockKeys.list });
			void client.invalidateQueries({ queryKey: subscriptionKeys.all });
			for (const key of AFFECTED) void client.invalidateQueries({ queryKey: [key] });
			// Tabs read the first page from the channel details, so those must be fresh first.
			void client
				.invalidateQueries({ queryKey: ['channel'] })
				.then(() => client.invalidateQueries({ queryKey: ['channel-tab'] }));
		}
	}));
}
