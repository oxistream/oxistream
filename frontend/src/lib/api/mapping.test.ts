import { describe, expect, it } from 'vitest';
import { ApiError, isNotFound, pickImage, toVideo, uniqueBy, unwrap } from './mapping';

const image = (width: number) => ({ url: `/proxy/image/${width}`, width, height: width });

describe('pickImage', () => {
	it('takes the smallest image that is wide enough', () => {
		expect(pickImage([image(720), image(168), image(360)], 300)).toBe('/proxy/image/360');
	});

	it('falls back to the largest image', () => {
		expect(pickImage([image(88), image(176)], 480)).toBe('/proxy/image/176');
		expect(pickImage([], 480)).toBeUndefined();
		expect(pickImage(undefined, 480)).toBeUndefined();
	});
});

describe('toVideo', () => {
	const base = {
		id: 'dQw4w9WgXcQ',
		title: 'Title',
		thumbnails: [image(360), image(720)],
		is_live: false,
		is_short: false,
		is_upcoming: false
	};

	it('maps nulls to missing fields and uses the fallback channel', () => {
		const video = toVideo(
			{ ...base, duration_seconds: null, view_count: null, published_at: null, channel: null },
			{ id: 'UC1', name: 'Owner' }
		);
		expect(video).toMatchObject({ id: 'dQw4w9WgXcQ', channel: { id: 'UC1', name: 'Owner' } });
		expect(video.durationSeconds).toBeUndefined();
		expect(video.viewCount).toBeUndefined();
		expect(video.thumbnailUrl).toBe('/proxy/image/720');
	});

	it('drops the duration of live streams', () => {
		expect(
			toVideo({ ...base, is_live: true, duration_seconds: 0 }).durationSeconds
		).toBeUndefined();
	});
});

describe('unwrap', () => {
	it('returns data and throws ApiError with the backend code', () => {
		expect(unwrap({ data: 1, response: new Response(null, { status: 200 }) })).toBe(1);
		try {
			unwrap({
				error: { code: 'not_found', message: 'gone' },
				response: new Response(null, { status: 404 })
			});
			expect.unreachable();
		} catch (error) {
			expect(error).toBeInstanceOf(ApiError);
			expect(isNotFound(error)).toBe(true);
			expect((error as ApiError).message).toBe('gone');
		}
	});
});

describe('uniqueBy', () => {
	it('keeps the first occurrence', () => {
		expect(uniqueBy(['a', 'b', 'a', 'c', 'b'], (x) => x)).toEqual(['a', 'b', 'c']);
	});
});
