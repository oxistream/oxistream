import { pickImage } from './mapping';
import type { components } from './schema';

export type SubscribedChannel = {
	id: string;
	/** Missing until the feed worker fetched the channel once. */
	name?: string;
	handle?: string;
	avatarUrl?: string;
	subscriberCount?: number;
	verified: boolean;
	subscribedAt: string;
	/** Uploads since the user last opened the channel's page. */
	hasNew: boolean;
};

export function toSubscribedChannel(
	channel: components['schemas']['SubscribedChannel']
): SubscribedChannel {
	return {
		id: channel.id,
		name: channel.name?.trim() || undefined,
		handle: channel.handle ?? undefined,
		avatarUrl: pickImage(channel.avatar, 88),
		subscriberCount: channel.subscriber_count ?? undefined,
		verified: channel.verified,
		subscribedAt: channel.subscribed_at,
		hasNew: channel.has_new
	};
}

/** By name like the backend, channels that were not fetched yet last. */
export function compareSubscriptions(a: SubscribedChannel, b: SubscribedChannel): number {
	if (a.name && b.name) {
		return (
			a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }) || a.id.localeCompare(b.id)
		);
	}
	if (a.name) return -1;
	if (b.name) return 1;
	return a.id.localeCompare(b.id);
}

/** Channels with new uploads first, each group by name in the given locale. */
export function sortForNav(list: SubscribedChannel[], locale: string): SubscribedChannel[] {
	const collator = new Intl.Collator(locale, { sensitivity: 'base' });
	return list.toSorted((a, b) => {
		if (a.hasNew !== b.hasNew) return a.hasNew ? -1 : 1;
		if (a.name && b.name) return collator.compare(a.name, b.name) || a.id.localeCompare(b.id);
		if (a.name) return -1;
		if (b.name) return 1;
		return a.id.localeCompare(b.id);
	});
}

export function withVisit(list: SubscribedChannel[], id: string): SubscribedChannel[] {
	return list.map((c) => (c.id === id && c.hasNew ? { ...c, hasNew: false } : c));
}

export function withSubscription(
	list: SubscribedChannel[],
	channel: SubscribedChannel
): SubscribedChannel[] {
	return [...list.filter((c) => c.id !== channel.id), channel].sort(compareSubscriptions);
}

export function withoutSubscription(list: SubscribedChannel[], id: string): SubscribedChannel[] {
	return list.filter((c) => c.id !== id);
}

export function hasPendingChannels(list: SubscribedChannel[]): boolean {
	return list.some((c) => !c.name);
}
