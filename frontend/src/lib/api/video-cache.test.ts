import { describe, expect, it } from 'vitest';
import { isHidingWatched, normalizeFilters, subscriptionKeys } from './subscriptions';
import { withoutVideo } from './video-cache';

describe('withoutVideo', () => {
	it('drops the video from feed pages and plain lists, keeping everything else', () => {
		const feed = {
			pages: [
				{ items: [{ id: 'a' }, { id: 'b' }], continuation: 'next' },
				{ items: [{ id: 'c' }] }
			],
			pageParams: [undefined, 'next']
		};
		expect(withoutVideo(feed, 'b')).toEqual({
			pages: [{ items: [{ id: 'a' }], continuation: 'next' }, { items: [{ id: 'c' }] }],
			pageParams: [undefined, 'next']
		});
		expect(withoutVideo([{ id: 'a' }, { id: 'b' }], 'a')).toEqual([{ id: 'b' }]);
		expect(withoutVideo(undefined, 'a')).toBeUndefined();
	});
});

describe('feed filters', () => {
	it('share one cache entry while unfiltered', () => {
		expect(subscriptionKeys.feedPages('videos', {})).toEqual(
			subscriptionKeys.feedPages('videos', { hideWatched: false, channel: '' })
		);
		expect(normalizeFilters({ hideWatched: true, channel: 'UCx' })).toEqual({
			hideWatched: true,
			channel: 'UCx'
		});
	});

	it('tells feeds that hide watched videos apart', () => {
		expect(isHidingWatched(subscriptionKeys.feedPages('videos', { hideWatched: true }))).toBe(true);
		expect(isHidingWatched(subscriptionKeys.feedPages('videos', { channel: 'UCx' }))).toBe(false);
		expect(isHidingWatched(['trending', 'news'])).toBe(false);
	});
});
