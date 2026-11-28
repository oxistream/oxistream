import { describe, expect, it } from 'vitest';
import { indexAt, toChapters } from './chapters';

describe('toChapters', () => {
	it('sorts chapters and ends each at the next one or the video', () => {
		const chapters = toChapters(
			[
				{ title: 'Two', start_seconds: 60, thumbnails: [] },
				{ title: 'One', start_seconds: 0, thumbnails: [] },
				{ title: 'Past the end', start_seconds: 900, thumbnails: [] }
			],
			120
		);
		expect(chapters.map((c) => [c.title, c.start, c.end])).toEqual([
			['One', 0, 60],
			['Two', 60, 120]
		]);
	});

	it('handles missing chapters and an unknown duration', () => {
		expect(toChapters(undefined)).toEqual([]);
		expect(toChapters([{ title: 'Only', start_seconds: 5, thumbnails: [] }])[0].end).toBe(Infinity);
	});
});

describe('indexAt', () => {
	const items = [{ start: 0 }, { start: 10 }, { start: 20 }];

	it('finds the item playing at a time', () => {
		expect(indexAt(items, 0)).toBe(0);
		expect(indexAt(items, 9.99)).toBe(0);
		expect(indexAt(items, 10)).toBe(1);
		expect(indexAt(items, 500)).toBe(2);
	});

	it('is -1 before the first item or without items', () => {
		expect(indexAt([{ start: 5 }], 4)).toBe(-1);
		expect(indexAt([], 4)).toBe(-1);
	});
});
