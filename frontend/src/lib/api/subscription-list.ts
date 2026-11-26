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
		subscribedAt: channel.subscribed_at
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
