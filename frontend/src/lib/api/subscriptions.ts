import {
	createInfiniteQuery,
	createMutation,
	createQuery,
	useQueryClient
} from '@tanstack/svelte-query';
import { session } from '#lib/session.svelte.js';
import type { VideoSummary } from '#lib/video.js';
import { blockKeys } from './blocks';
import { api } from './client';
import { toShort, toVideo, unwrap } from './mapping';
import { groupKeys } from './subscription-group-list';
import type { operations } from './schema';
import {
	hasPendingChannels,
	toSubscribedChannel,
	withoutSubscription,
	withSubscription,
	type SubscribedChannel
} from './subscription-list';

export type FeedPage = { items: VideoSummary[]; continuation?: string };
export type FeedKind = Exclude<
	NonNullable<operations['feed']['parameters']['query']>['kind'],
	'all' | undefined
>;

/** Narrows the subscription feed; the backend applies it, so pages stay full. */
export type FeedFilters = { hideWatched?: boolean; channel?: string; group?: string };

const FEED_PAGE_SIZE = 30;
// While a new channel waits for its first fetch, the list and feed are polled until it lands.
const PENDING_POLL_MS = 4000;

export const subscriptionKeys = {
	all: ['subscriptions'] as const,
	list: ['subscriptions', 'list'] as const,
	state: (channelId: string) => ['subscriptions', 'state', channelId] as const,
	feed: ['feed'] as const,
	/** Unfiltered feeds share `{}`, e.g. the home row and the feed page without filters. */
	feedPages: (kind: FeedKind, filters: FeedFilters) =>
		['feed', kind, normalizeFilters(filters)] as const
};

export function normalizeFilters(filters: FeedFilters): FeedFilters {
	return {
		...(filters.hideWatched ? { hideWatched: true } : {}),
		...(filters.channel ? { channel: filters.channel } : {}),
		...(filters.group ? { group: filters.group } : {})
	};
}

/** Feed pages that leave out watched videos, so marking a video changes them. */
export function isHidingWatched(queryKey: readonly unknown[]): boolean {
	const filters = queryKey[2] as FeedFilters | undefined;
	return queryKey[0] === subscriptionKeys.feed[0] && filters?.hideWatched === true;
}

async function fetchSubscriptions(signal: AbortSignal): Promise<SubscribedChannel[]> {
	const list = unwrap(await api.GET('/api/v0/subscriptions', { signal }));
	return list.map(toSubscribedChannel);
}

/** The signed-in user's channels; idle for anonymous users. */
export function createSubscriptionsQuery() {
	return createQuery(() => ({
		queryKey: subscriptionKeys.list,
		queryFn: ({ signal }: { signal: AbortSignal }) => fetchSubscriptions(signal),
		enabled: session.user !== null,
		staleTime: 60_000,
		refetchInterval: (query: { state: { data?: SubscribedChannel[] } }) =>
			query.state.data && hasPendingChannels(query.state.data) ? PENDING_POLL_MS : false
	}));
}

export function createSubscriptionStateQuery(channelId: () => string | undefined) {
	return createQuery(() => {
		const id = channelId() ?? '';
		return {
			queryKey: subscriptionKeys.state(id),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/subscriptions/{channel_id}', {
						params: { path: { channel_id: id } },
						signal
					})
				).subscribed,
			enabled: session.user !== null && id.length > 0,
			staleTime: 60_000
		};
	});
}

async function fetchFeedPage(
	kind: FeedKind,
	filters: FeedFilters,
	continuation: string | undefined,
	signal: AbortSignal
): Promise<FeedPage> {
	const query = {
		limit: FEED_PAGE_SIZE,
		kind,
		continuation,
		hide_watched: filters.hideWatched || undefined,
		channel: filters.channel,
		group: filters.group
	};
	const page = unwrap(await api.GET('/api/v0/feed', { params: { query }, signal }));
	return {
		items: page.items.map((video) => (kind === 'shorts' ? toShort(video) : toVideo(video))),
		continuation: page.continuation ?? undefined
	};
}

/**
 * Videos or shorts of subscribed channels, newest first; polls while channels are still being
 * fetched. Regular videos and shorts are listed apart, as grid and shelf.
 */
export function createFeedQuery(
	polling: () => boolean,
	kind: FeedKind = 'videos',
	options: { filters?: () => FeedFilters; enabled?: () => boolean } = {}
) {
	return createInfiniteQuery(() => {
		const filters = normalizeFilters(options.filters?.() ?? {});
		return {
			queryKey: subscriptionKeys.feedPages(kind, filters),
			queryFn: ({ pageParam, signal }: { pageParam: string | undefined; signal: AbortSignal }) =>
				fetchFeedPage(kind, filters, pageParam, signal),
			initialPageParam: undefined as string | undefined,
			getNextPageParam: (last: FeedPage) => last.continuation,
			enabled: session.user !== null && (options.enabled?.() ?? true),
			staleTime: 60_000,
			refetchInterval: polling() ? PENDING_POLL_MS : false
		};
	});
}

type SubscriptionChange = { channelId: string; subscribe: boolean };

/** Subscribes or unsubscribes, updating the button and the channel list right away. */
export function createSubscriptionMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ channelId, subscribe }: SubscriptionChange) => {
			const params = { params: { path: { channel_id: channelId } } };
			if (subscribe) {
				return toSubscribedChannel(
					unwrap(await api.PUT('/api/v0/subscriptions/{channel_id}', params))
				);
			}
			const { response, error } = await api.DELETE('/api/v0/subscriptions/{channel_id}', params);
			if (!response.ok) unwrap({ error, response });
			return null;
		},
		onMutate: async ({ channelId, subscribe }: SubscriptionChange) => {
			const stateKey = subscriptionKeys.state(channelId);
			await client.cancelQueries({ queryKey: stateKey });
			await client.cancelQueries({ queryKey: subscriptionKeys.list });
			const previous = {
				state: client.getQueryData<boolean>(stateKey),
				list: client.getQueryData<SubscribedChannel[]>(subscriptionKeys.list)
			};
			client.setQueryData(stateKey, subscribe);
			if (!subscribe && previous.list) {
				client.setQueryData(subscriptionKeys.list, withoutSubscription(previous.list, channelId));
			}
			return previous;
		},
		onError: (
			_error: Error,
			{ channelId }: SubscriptionChange,
			previous?: { state?: boolean; list?: SubscribedChannel[] }
		) => {
			if (!previous) return;
			client.setQueryData(subscriptionKeys.state(channelId), previous.state);
			if (previous.list) client.setQueryData(subscriptionKeys.list, previous.list);
		},
		onSuccess: (channel: SubscribedChannel | null) => {
			if (!channel) return;
			client.setQueryData<SubscribedChannel[]>(subscriptionKeys.list, (list) =>
				list ? withSubscription(list, channel) : list
			);
		},
		// Not awaited, so the button settles without waiting for the refetch.
		onSettled: (
			_data: SubscribedChannel | null | undefined,
			_error: Error | null,
			{ subscribe }: SubscriptionChange
		) => {
			void client.invalidateQueries({ queryKey: subscriptionKeys.all });
			void client.invalidateQueries({ queryKey: subscriptionKeys.feed });
			// Unsubscribing removes the channel from its groups.
			if (!subscribe) void client.invalidateQueries({ queryKey: groupKeys.list });
			// Subscribing lifts a block of the channel.
			if (subscribe) void client.invalidateQueries({ queryKey: blockKeys.list });
		}
	}));
}
