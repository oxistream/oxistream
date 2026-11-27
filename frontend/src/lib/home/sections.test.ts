import { describe, expect, it } from 'vitest';
import {
	editableSections,
	enabledSections,
	homeSections,
	rowState,
	showGuestFallback,
	trendingCategory
} from './sections';

describe('home sections', () => {
	it('defaults per audience', () => {
		expect(homeSections('user')).toEqual([
			'continue-watching',
			'subscriptions',
			'subscription-shorts'
		]);
		expect(homeSections('guest')).toEqual([
			'trending-news',
			'trending-gaming',
			'trending-podcasts'
		]);
	});

	it('keeps the chosen order and drops unknown and duplicate keys', () => {
		expect(
			homeSections('user', ['subscriptions', 'bogus', 'trending-news', 'subscriptions'])
		).toEqual(['subscriptions', 'trending-news']);
		expect(homeSections('user', [])).toEqual([]);
		expect(homeSections('user', null)).toEqual(homeSections('user'));
	});

	it('always gives guests their default', () => {
		expect(homeSections('guest', ['subscriptions', 'trending-podcasts'])).toEqual([
			'trending-news',
			'trending-gaming',
			'trending-podcasts'
		]);
	});

	it('lists every section for editing, the disabled ones last', () => {
		const sections = editableSections(['trending-gaming', 'continue-watching']);
		expect(sections.map((s) => [s.key, s.enabled])).toEqual([
			['trending-gaming', true],
			['continue-watching', true],
			['subscriptions', false],
			['subscription-shorts', false],
			['trending-news', false],
			['trending-podcasts', false],
			['trending-music', false],
			['trending-sports', false],
			['trending-learning', false],
			['trending-fashion', false],
			['trending-live', false]
		]);
		expect(enabledSections(sections)).toEqual(['trending-gaming', 'continue-watching']);
		expect(enabledSections(editableSections())).toEqual(homeSections('user'));
	});

	it('maps trending sections to their category', () => {
		expect(trendingCategory('trending-gaming')).toBe('gaming');
		expect(trendingCategory('trending-fashion')).toBe('fashion');
		expect(trendingCategory('subscriptions')).toBeUndefined();
	});
});

describe('row state', () => {
	it('loads, shows and hides', () => {
		expect(rowState({ isPending: true, count: 0 })).toBe('loading');
		expect(rowState({ isPending: false, count: 3 })).toBe('shown');
		expect(rowState({ isPending: false, count: 0 })).toBe('hidden');
	});

	it('falls back for guests only once every row settled without videos', () => {
		expect(showGuestFallback({ pending: true, withVideos: 0 })).toBe(false);
		expect(showGuestFallback({ pending: true, withVideos: 1 })).toBe(false);
		expect(showGuestFallback({ pending: false, withVideos: 1 })).toBe(false);
		expect(showGuestFallback({ pending: false, withVideos: 0 })).toBe(true);
	});
});
