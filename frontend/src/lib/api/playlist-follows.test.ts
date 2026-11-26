import { describe, expect, it } from 'vitest';
import { hasPendingPlaylists, toFollowedPlaylist } from './playlist-follows';

const base = {
	id: 'PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI',
	thumbnails: [
		{ url: '/proxy/image/small', width: 168, height: 94 },
		{ url: '/proxy/image/large', width: 336, height: 188 }
	],
	new_video_count: 2,
	unavailable: false,
	followed_at: '2026-11-26T10:00:00Z'
};

describe('toFollowedPlaylist', () => {
	it('maps a fetched playlist', () => {
		const playlist = toFollowedPlaylist({
			...base,
			title: 'Road trips',
			channel: { id: 'UCx', name: 'North', avatar: [], verified: false },
			video_count: 240
		});
		expect(playlist).toEqual({
			id: base.id,
			title: 'Road trips',
			channel: { id: 'UCx', name: 'North' },
			thumbnailUrl: '/proxy/image/large',
			videoCount: 240,
			newVideoCount: 2,
			unavailable: false,
			followedAt: base.followed_at
		});
	});

	it('leaves unknown fields out until the first fetch', () => {
		const playlist = toFollowedPlaylist({
			...base,
			title: null,
			thumbnails: [],
			video_count: null
		});
		expect(playlist.title).toBeUndefined();
		expect(playlist.thumbnailUrl).toBeUndefined();
		expect(playlist.videoCount).toBeUndefined();
	});
});

describe('hasPendingPlaylists', () => {
	it('is true only while a playlist waits for its first fetch', () => {
		const fetched = toFollowedPlaylist({ ...base, title: 'A' });
		const pending = toFollowedPlaylist({ ...base, title: null });
		const failed = toFollowedPlaylist({ ...base, title: null, unavailable: true });
		expect(hasPendingPlaylists([fetched])).toBe(false);
		expect(hasPendingPlaylists([fetched, pending])).toBe(true);
		expect(hasPendingPlaylists([failed])).toBe(false);
	});
});
