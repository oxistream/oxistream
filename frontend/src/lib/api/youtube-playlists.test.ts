import { describe, expect, it } from 'vitest';
import { youtubePlayback } from '#lib/components/playlist/playback.js';
import { toVideo } from './mapping';
import type { components } from './schema';
import {
	isYouTubePlaylistId,
	mergeYouTubePlaylist,
	toYouTubePlaylistPage,
	type YouTubePlaylistPage
} from './youtube-playlists';

type Summary = components['schemas']['VideoSummary'];

function summary(id: string): Summary {
	return {
		id,
		title: `Video ${id}`,
		thumbnails: [{ url: `/proxy/image/${id}`, width: 480, height: 360 }],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: 60,
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

const first = toYouTubePlaylistPage({
	id: 'PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI',
	title: 'Coastal drives',
	description: 'Long roads',
	channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
	video_count: 5,
	thumbnails: [{ url: '/proxy/image/cover', width: 336, height: 188 }],
	last_update: '2026-11-20',
	last_update_text: 'Last updated on Nov 20, 2026',
	videos: { items: [summary('a'), summary('b')], continuation: 'next' }
});

describe('toYouTubePlaylistPage', () => {
	it('maps the playlist and its first page', () => {
		expect(first.playlist).toEqual({
			id: 'PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI',
			title: 'Coastal drives',
			description: 'Long roads',
			channel: { id: 'UCnorth', name: 'Northbound' },
			videoCount: 5,
			thumbnailUrl: '/proxy/image/cover',
			lastUpdate: '2026-11-20',
			lastUpdateText: 'Last updated on Nov 20, 2026'
		});
		expect(first.items.map((v) => v.id)).toEqual(['a', 'b']);
		expect(first.items[0].channel.name).toBe('Northbound');
		expect(first.continuation).toBe('next');
	});
});

describe('mergeYouTubePlaylist', () => {
	const second: YouTubePlaylistPage = {
		items: [summary('b'), summary('c')].map((v) => toVideo(v))
	};

	it('joins pages and drops repeated videos', () => {
		const merged = mergeYouTubePlaylist({
			pages: [first, second],
			pageParams: [undefined, 'next']
		});
		expect(merged?.title).toBe('Coastal drives');
		expect(merged?.items.map((v) => v.id)).toEqual(['a', 'b', 'c']);
	});

	it('is empty without a first page', () => {
		expect(mergeYouTubePlaylist(undefined)).toBeUndefined();
		expect(mergeYouTubePlaylist({ pages: [], pageParams: [] })).toBeUndefined();
	});
});

describe('youtubePlayback', () => {
	const merged = mergeYouTubePlaylist({ pages: [first], pageParams: [undefined] })!;

	it('counts YouTube’s total while pages remain, else what was listed', () => {
		expect(youtubePlayback(merged, true)).toMatchObject({ owner: 'Northbound', total: 5 });
		expect(youtubePlayback(merged, false).total).toBe(2);
		expect(youtubePlayback(merged, true).visibility).toBeUndefined();
	});
});

describe('isYouTubePlaylistId', () => {
	it('excludes local IDs and mixes', () => {
		expect(isYouTubePlaylistId('PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI')).toBe(true);
		expect(isYouTubePlaylistId('UUBJycsmduvYEL83R_U4JriQ')).toBe(true);
		expect(isYouTubePlaylistId('Rd7xQ2mKp9LsVb3N')).toBe(false);
		expect(isYouTubePlaylistId('RDdQw4w9WgXcQ')).toBe(false);
		expect(isYouTubePlaylistId('')).toBe(false);
		expect(isYouTubePlaylistId(null)).toBe(false);
	});
});
