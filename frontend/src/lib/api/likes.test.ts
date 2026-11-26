import { describe, expect, it } from 'vitest';
import { likedPlayback } from '#lib/components/playlist/playback.js';
import { mergeLikedVideos, toLikedPage, type LikedPage } from './likes';
import type { components } from './schema';

function entry(id: string, likedAt: string): components['schemas']['LikedVideoEntry'] {
	return {
		video: {
			id,
			title: `Video ${id}`,
			thumbnails: [{ url: `/proxy/image/${id}`, width: 480, height: 360 }],
			channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
			duration_seconds: 60,
			is_live: false,
			is_short: false,
			is_upcoming: false
		},
		liked_at: likedAt
	};
}

const first = toLikedPage({
	items: [entry('b', '2026-11-26T10:00:00Z'), entry('a', '2026-11-25T10:00:00Z')],
	total: 3,
	continuation: 'next'
});

describe('toLikedPage', () => {
	it('maps videos and keeps when they were liked', () => {
		expect(first.items.map((v) => [v.id, v.likedAt, v.thumbnailUrl])).toEqual([
			['b', '2026-11-26T10:00:00Z', '/proxy/image/b'],
			['a', '2026-11-25T10:00:00Z', '/proxy/image/a']
		]);
		expect(first.total).toBe(3);
		expect(first.continuation).toBe('next');
	});
});

describe('mergeLikedVideos', () => {
	it('is undefined until the first page arrived', () => {
		expect(mergeLikedVideos(undefined)).toBeUndefined();
		expect(mergeLikedVideos({ pages: [], pageParams: [] })).toBeUndefined();
	});

	it('joins pages and drops videos that moved between pages', () => {
		const second: LikedPage = toLikedPage({
			items: [entry('a', '2026-11-25T10:00:00Z'), entry('c', '2026-11-24T10:00:00Z')],
			total: 3,
			continuation: null
		});
		const merged = mergeLikedVideos({ pages: [first, second], pageParams: [undefined, 'next'] });
		expect(merged?.items.map((v) => v.id)).toEqual(['b', 'a', 'c']);
		expect(merged?.total).toBe(3);
	});

	it('plays as a private list of the user', () => {
		const merged = mergeLikedVideos({ pages: [first], pageParams: [undefined] });
		const playback = merged && likedPlayback(merged, 'mara');
		expect(playback).toMatchObject({ id: 'LL', owner: 'mara', visibility: 'private', total: 3 });
		expect(playback?.items).toHaveLength(2);
	});
});
