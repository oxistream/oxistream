import { describe, expect, it } from 'vitest';
import {
	dropIndex,
	isLocalPlaylistId,
	isMixId,
	moveItem,
	needsMoreItems,
	neighbors,
	resumeVideo
} from './playlist-order';

const items = ['a', 'b', 'c', 'd'].map((id) => ({ id }));
const ids = (list: { id: string }[]) => list.map((item) => item.id).join('');

describe('isLocalPlaylistId', () => {
	it('accepts our IDs and rejects YouTube ones', () => {
		expect(isLocalPlaylistId('Ab3_-xYz09QwErTy')).toBe(true);
		for (const id of [
			'PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf',
			'RDdQw4w9WgXcQ',
			'OLAK5uy_kZ8ow6xHp',
			'short',
			'with space 12345',
			'',
			null,
			undefined
		]) {
			expect(isLocalPlaylistId(id)).toBe(false);
		}
	});
});

describe('moveItem', () => {
	it('moves forwards and backwards without touching the input', () => {
		expect(ids(moveItem(items, 0, 2))).toBe('bcad');
		expect(ids(moveItem(items, 3, 0))).toBe('dabc');
		expect(ids(moveItem(items, 1, 1))).toBe('abcd');
		expect(ids(items)).toBe('abcd');
	});

	it('clamps the target and ignores unknown sources', () => {
		expect(ids(moveItem(items, 0, 99))).toBe('bcda');
		expect(ids(moveItem(items, 2, -5))).toBe('cabd');
		expect(ids(moveItem(items, 7, 0))).toBe('abcd');
	});
});

describe('dropIndex', () => {
	const midpoints = [10, 30, 50, 70];

	it('counts the other rows above the pointer', () => {
		expect(dropIndex(midpoints, 0, 5)).toBe(0);
		expect(dropIndex(midpoints, 0, 35)).toBe(1);
		expect(dropIndex(midpoints, 0, 99)).toBe(3);
		expect(dropIndex(midpoints, 3, 0)).toBe(0);
		expect(dropIndex(midpoints, 3, 45)).toBe(2);
		expect(dropIndex(midpoints, 1, 30)).toBe(1);
	});

	it('round-trips with moveItem', () => {
		expect(ids(moveItem(items, 3, dropIndex(midpoints, 3, 20)))).toBe('adbc');
	});
});

describe('neighbors', () => {
	it('finds the items around the current one', () => {
		expect(neighbors(items, 'b')).toEqual({ index: 1, previous: { id: 'a' }, next: { id: 'c' } });
		expect(neighbors(items, 'a')).toEqual({ index: 0, previous: undefined, next: { id: 'b' } });
		expect(neighbors(items, 'd').next).toBeUndefined();
	});

	it('reports videos outside the playlist', () => {
		expect(neighbors(items, 'x')).toEqual({ index: -1 });
		expect(neighbors(items, null)).toEqual({ index: -1 });
	});
});

describe('resumeVideo', () => {
	it('continues where the owner left off, else from the start', () => {
		expect(resumeVideo(items, 'c')).toEqual({ id: 'c' });
		expect(resumeVideo(items, 'gone')).toEqual({ id: 'a' });
		expect(resumeVideo(items, null)).toEqual({ id: 'a' });
		expect(resumeVideo([], 'a')).toBeUndefined();
	});
});

describe('isMixId', () => {
	it('tells generated mixes from curated RDCLAK lists', () => {
		expect(isMixId('RDdQw4w9WgXcQ')).toBe(true);
		expect(isMixId('RDMMdQw4w9WgXcQ')).toBe(true);
		expect(isMixId('RDCLAK5uy_kFQXdnqMaQCVx2wpUM4ZfbsGCDibZtkJk')).toBe(false);
		expect(isMixId('PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf')).toBe(false);
		expect(isMixId(null)).toBe(false);
	});
});

describe('needsMoreItems', () => {
	it('asks for more until the current video and its successors are loaded', () => {
		expect(needsMoreItems(items, 'a')).toBe(false);
		expect(needsMoreItems(items, 'b')).toBe(false);
		expect(needsMoreItems(items, 'c')).toBe(true);
		expect(needsMoreItems(items, 'd')).toBe(true);
		expect(needsMoreItems(items, 'later')).toBe(true);
		expect(needsMoreItems(items, 'b', 3)).toBe(true);
		expect(needsMoreItems(items, null)).toBe(false);
	});
});
