import { describe, expect, it } from 'vitest';
import { homeSections, rowState, showGuestFallback, trendingCategory } from './sections';

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

	it('keeps a chosen order and drops unknown, duplicate and foreign keys', () => {
		expect(
			homeSections('user', ['subscriptions', 'bogus', 'trending-news', 'subscriptions'])
		).toEqual(['subscriptions']);
		expect(homeSections('guest', ['trending-podcasts', 'trending-news'])).toEqual([
			'trending-podcasts',
			'trending-news'
		]);
		expect(homeSections('user', [])).toEqual([]);
	});

	it('maps trending sections to their category', () => {
		expect(trendingCategory('trending-gaming')).toBe('gaming');
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
