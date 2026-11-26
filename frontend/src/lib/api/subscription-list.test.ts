import { describe, expect, it } from 'vitest';
import {
	compareSubscriptions,
	hasPendingChannels,
	toSubscribedChannel,
	withoutSubscription,
	withSubscription,
	type SubscribedChannel
} from './subscription-list';

const channel = (id: string, name?: string): SubscribedChannel => ({
	id,
	name,
	verified: false,
	subscribedAt: '2026-11-26T10:00:00Z'
});

describe('toSubscribedChannel', () => {
	it('maps the API shape and treats blank names as not fetched', () => {
		expect(
			toSubscribedChannel({
				id: 'UCa',
				name: '  ',
				handle: null,
				avatar: [{ url: '/proxy/image/a', width: 88, height: 88 }],
				subscriber_count: 1200,
				verified: true,
				subscribed_at: '2026-11-26T10:00:00Z'
			})
		).toEqual({
			id: 'UCa',
			name: undefined,
			handle: undefined,
			avatarUrl: '/proxy/image/a',
			subscriberCount: 1200,
			verified: true,
			subscribedAt: '2026-11-26T10:00:00Z'
		});
	});
});

describe('compareSubscriptions', () => {
	it('sorts by name ignoring case, unfetched channels last', () => {
		const list = [channel('UCc'), channel('UCb', 'bench'), channel('UCa', 'Anvil'), channel('UCd')];
		expect(list.sort(compareSubscriptions).map((c) => c.id)).toEqual(['UCa', 'UCb', 'UCc', 'UCd']);
	});
});

describe('withSubscription', () => {
	it('inserts in order and replaces an existing entry', () => {
		const list = [channel('UCa', 'Anvil'), channel('UCc', 'Cello')];
		expect(withSubscription(list, channel('UCb', 'Bench')).map((c) => c.id)).toEqual([
			'UCa',
			'UCb',
			'UCc'
		]);
		const renamed = withSubscription(list, channel('UCa', 'Zither'));
		expect(renamed.map((c) => c.name)).toEqual(['Cello', 'Zither']);
	});
});

describe('withoutSubscription', () => {
	it('drops the channel', () => {
		expect(withoutSubscription([channel('UCa'), channel('UCb')], 'UCa')).toEqual([channel('UCb')]);
	});
});

describe('hasPendingChannels', () => {
	it('is true while a channel has no name yet', () => {
		expect(hasPendingChannels([channel('UCa', 'Anvil')])).toBe(false);
		expect(hasPendingChannels([channel('UCa', 'Anvil'), channel('UCb')])).toBe(true);
		expect(hasPendingChannels([])).toBe(false);
	});
});
