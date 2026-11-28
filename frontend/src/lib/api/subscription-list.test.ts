import { describe, expect, it } from 'vitest';
import {
	compareSubscriptions,
	hasPendingChannels,
	sortForNav,
	toSubscribedChannel,
	withVisit,
	withoutSubscription,
	withSubscription,
	type SubscribedChannel
} from './subscription-list';

const channel = (id: string, name?: string): SubscribedChannel => ({
	id,
	name,
	verified: false,
	subscribedAt: '2026-11-26T10:00:00Z',
	hasNew: false
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
				subscribed_at: '2026-11-26T10:00:00Z',
				has_new: true
			})
		).toEqual({
			id: 'UCa',
			name: undefined,
			handle: undefined,
			avatarUrl: '/proxy/image/a',
			subscriberCount: 1200,
			verified: true,
			subscribedAt: '2026-11-26T10:00:00Z',
			hasNew: true
		});
	});
});

describe('compareSubscriptions', () => {
	it('sorts by name ignoring case, unfetched channels last', () => {
		const list = [channel('UCc'), channel('UCb', 'bench'), channel('UCa', 'Anvil'), channel('UCd')];
		expect(list.sort(compareSubscriptions).map((c) => c.id)).toEqual(['UCa', 'UCb', 'UCc', 'UCd']);
	});
});

describe('sortForNav', () => {
	const fresh = (id: string, name?: string) => ({ ...channel(id, name), hasNew: true });

	it('puts new uploads first, both groups by name', () => {
		const list = [
			channel('UCa', 'anvil'),
			fresh('UCz', 'Zither'),
			channel('UCp'),
			channel('UCb', 'Bench'),
			fresh('UCc', 'cello')
		];
		expect(sortForNav(list, 'en').map((c) => c.id)).toEqual(['UCc', 'UCz', 'UCa', 'UCb', 'UCp']);
	});

	it('follows the locale', () => {
		const list = [channel('UCo', 'Ozean'), channel('UCu', 'Übersee'), channel('UCa', 'Äther')];
		expect(sortForNav(list, 'de').map((c) => c.name)).toEqual(['Äther', 'Ozean', 'Übersee']);
	});
});

describe('withVisit', () => {
	it('clears only the visited channel', () => {
		const list = [
			{ ...channel('UCa'), hasNew: true },
			{ ...channel('UCb'), hasNew: true }
		];
		expect(withVisit(list, 'UCa').map((c) => c.hasNew)).toEqual([false, true]);
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
