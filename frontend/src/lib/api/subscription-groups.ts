import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { session } from '#lib/session.svelte.js';
import { api } from './client';
import { unwrap } from './mapping';
import {
	groupKeys,
	toSubscriptionGroup,
	withFields,
	withGroupMoved,
	withMembership,
	type GroupFields,
	type SubscriptionGroup
} from './subscription-group-list';
import { subscriptionKeys } from './subscriptions';

type Client = ReturnType<typeof useQueryClient>;

/** The signed-in user's groups, in their order; idle for anonymous users. */
export function createSubscriptionGroupsQuery() {
	return createQuery(() => ({
		queryKey: groupKeys.list,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/subscription-groups', { signal })).map(toSubscriptionGroup),
		enabled: session.user !== null,
		staleTime: 60_000
	}));
}

/** Applies `change` to the cached list right away and rolls it back on failure. */
type Snapshot = { previous?: SubscriptionGroup[] };

async function optimistic(
	client: Client,
	change: (groups: SubscriptionGroup[]) => SubscriptionGroup[]
): Promise<Snapshot> {
	await client.cancelQueries({ queryKey: groupKeys.list });
	const previous = client.getQueryData<SubscriptionGroup[]>(groupKeys.list);
	if (previous) client.setQueryData(groupKeys.list, change(previous));
	return { previous };
}

function rollback(client: Client, snapshot: Snapshot | undefined) {
	if (snapshot?.previous) client.setQueryData(groupKeys.list, snapshot.previous);
}

function settle(client: Client, feed: boolean) {
	void client.invalidateQueries({ queryKey: groupKeys.list });
	if (feed) void client.invalidateQueries({ queryKey: subscriptionKeys.feed });
}

function ok(result: { response: Response; error?: unknown }) {
	if (!result.response.ok) unwrap({ ...result, data: undefined });
}

export function createCreateGroupMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ name, channelIds }: { name: string; channelIds?: string[] }) =>
			toSubscriptionGroup(
				unwrap(
					await api.POST('/api/v0/subscription-groups', {
						body: { name, channel_ids: channelIds ?? [] }
					})
				)
			),
		onSuccess: (group: SubscriptionGroup) => {
			client.setQueryData<SubscriptionGroup[]>(groupKeys.list, (list) =>
				list ? [...list.filter((g) => g.id !== group.id), group] : list
			);
		},
		onSettled: () => settle(client, false)
	}));
}

type GroupUpdate = { id: string; fields: GroupFields };

export function createUpdateGroupMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id, fields }: GroupUpdate) =>
			toSubscriptionGroup(
				unwrap(
					await api.PATCH('/api/v0/subscription-groups/{id}', {
						params: { path: { id } },
						body: { name: fields.name, show_on_home: fields.showOnHome }
					})
				)
			),
		onMutate: ({ id, fields }: GroupUpdate) =>
			optimistic(client, (groups) => withFields(groups, id, fields)),
		onError: (_error: Error, _change: GroupUpdate, snapshot?: Snapshot) =>
			rollback(client, snapshot),
		onSettled: () => settle(client, false)
	}));
}

export function createDeleteGroupMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (id: string) =>
			ok(await api.DELETE('/api/v0/subscription-groups/{id}', { params: { path: { id } } })),
		onMutate: (id: string) => optimistic(client, (groups) => groups.filter((g) => g.id !== id)),
		onError: (_error: Error, _id: string, snapshot?: Snapshot) => rollback(client, snapshot),
		onSettled: () => settle(client, false)
	}));
}

type GroupMove = { id: string; index: number };

export function createMoveGroupMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id, index }: GroupMove) =>
			ok(
				await api.PUT('/api/v0/subscription-groups/{id}/position', {
					params: { path: { id } },
					body: { index }
				})
			),
		onMutate: ({ id, index }: GroupMove) =>
			optimistic(client, (groups) => withGroupMoved(groups, id, index)),
		onError: (_error: Error, _move: GroupMove, snapshot?: Snapshot) => rollback(client, snapshot),
		onSettled: () => settle(client, false)
	}));
}

type MembershipChange = { groupId: string; channelId: string; member: boolean };

/** Adds a channel to a group or removes it, updating the list right away. */
export function createGroupMembershipMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ groupId, channelId, member }: MembershipChange) => {
			const params = { params: { path: { id: groupId, channel_id: channelId } } };
			const path = '/api/v0/subscription-groups/{id}/channels/{channel_id}';
			ok(member ? await api.PUT(path, params) : await api.DELETE(path, params));
		},
		onMutate: ({ groupId, channelId, member }: MembershipChange) =>
			optimistic(client, (groups) => withMembership(groups, groupId, channelId, member)),
		onError: (_error: Error, _change: MembershipChange, snapshot?: Snapshot) =>
			rollback(client, snapshot),
		onSettled: () => settle(client, true)
	}));
}
