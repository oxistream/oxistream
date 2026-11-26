import { describe, expect, it } from 'vitest';
import { toChannelPlaylist, toChannelVideo } from './channel';

const owner = { id: 'UC1', name: 'Forge' };
const short = {
	id: 'GBxwJxQbYl4',
	title: 'Quench',
	thumbnails: [
		{ url: '/proxy/image/wide', width: 480, height: 360 },
		{ url: '/proxy/image/tall', width: 404, height: 720 }
	],
	duration_seconds: null,
	view_count: 469_000,
	viewers: null,
	published_at: null,
	published_text: null,
	is_live: false,
	is_short: true,
	is_upcoming: false,
	channel: null,
	description: null
};

describe('toChannelVideo', () => {
	it('gives shorts their portrait thumbnail and the channel as owner', () => {
		const mapped = toChannelVideo(short, owner, 'shorts');
		expect(mapped.thumbnailUrl).toBe('/proxy/image/tall');
		expect(mapped.channel).toEqual(owner);
		expect(mapped.viewCount).toBe(469_000);
	});

	it('keeps landscape thumbnails on other tabs', () => {
		expect(toChannelVideo(short, owner, 'videos').thumbnailUrl).toBe('/proxy/image/wide');
	});
});

describe('toChannelPlaylist', () => {
	it('maps the playlist and leaves an unknown count out', () => {
		expect(
			toChannelPlaylist({
				id: 'PLforge',
				title: 'Forge basics',
				thumbnails: [{ url: '/proxy/image/pl', width: 480, height: 270 }],
				channel: null,
				video_count: null,
				is_course: false
			})
		).toEqual({
			id: 'PLforge',
			title: 'Forge basics',
			thumbnailUrl: '/proxy/image/pl',
			channel: undefined,
			videoCount: undefined,
			isCourse: false
		});
	});

	it('keeps the course flag', () => {
		const course = toChannelPlaylist({
			id: 'PLAscMa3kKhHjBYOTrT20kpYMNQX95KU6i',
			title: 'Rust for beginners',
			thumbnails: [],
			channel: null,
			video_count: 85,
			is_course: true
		});
		expect(course.isCourse).toBe(true);
		expect(course.videoCount).toBe(85);
	});
});
