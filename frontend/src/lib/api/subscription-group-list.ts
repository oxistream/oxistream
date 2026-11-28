import { moveItem } from './playlist-order';
import type { components } from './schema';

export const MAX_GROUPS = 50;
export const MAX_GROUP_NAME = 50;

export const groupKeys = {
	list: ['subscription-groups'] as const
};

export type SubscriptionGroup = {
	id: string;
	name: string;
	showOnHome: boolean;
	channelIds: string[];
};

export type GroupFields = { name?: string; showOnHome?: boolean };

export function toSubscriptionGroup(
	group: components['schemas']['SubscriptionGroup']
): SubscriptionGroup {
	return {
		id: group.id,
		name: group.name,
		showOnHome: group.show_on_home,
		channelIds: group.channel_ids
	};
}

export function isGroupId(id: string | null | undefined): id is string {
	return (
		typeof id === 'string' &&
		/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id)
	);
}

export function withMembership(
	groups: SubscriptionGroup[],
	groupId: string,
	channelId: string,
	member: boolean
): SubscriptionGroup[] {
	return groups.map((group) => {
		if (group.id !== groupId) return group;
		const others = group.channelIds.filter((id) => id !== channelId);
		return { ...group, channelIds: member ? [...others, channelId] : others };
	});
}

export function withFields(
	groups: SubscriptionGroup[],
	groupId: string,
	fields: GroupFields
): SubscriptionGroup[] {
	return groups.map((group) =>
		group.id === groupId
			? {
					...group,
					name: fields.name ?? group.name,
					showOnHome: fields.showOnHome ?? group.showOnHome
				}
			: group
	);
}

export function withGroupMoved(
	groups: SubscriptionGroup[],
	groupId: string,
	index: number
): SubscriptionGroup[] {
	return moveItem(
		groups,
		groups.findIndex((group) => group.id === groupId),
		index
	);
}

/** Whether another group already has this name, ignoring case like the backend. */
export function isNameTaken(groups: SubscriptionGroup[], name: string, exceptId?: string): boolean {
	const folded = name.trim().toLowerCase();
	return groups.some((group) => group.id !== exceptId && group.name.toLowerCase() === folded);
}
