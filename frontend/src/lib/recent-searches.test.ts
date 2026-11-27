import { describe, expect, it } from 'vitest';
import {
	MAX_RECENT_SEARCHES,
	addRecent,
	isRecentList,
	matchRecent,
	removeRecent
} from './recent-searches';

describe('recent searches', () => {
	it('adds to the front without duplicates', () => {
		expect(addRecent(['b', 'Forge'], '  forge  ')).toEqual(['forge', 'b']);
		expect(addRecent(['a'], 'forge   welding')).toEqual(['forge welding', 'a']);
		expect(addRecent(['a'], '   ')).toEqual(['a']);
	});

	it('keeps at most the maximum', () => {
		let list: string[] = [];
		for (let i = 0; i < MAX_RECENT_SEARCHES + 5; i++) list = addRecent(list, `q${i}`);
		expect(list).toHaveLength(MAX_RECENT_SEARCHES);
		expect(list[0]).toBe(`q${MAX_RECENT_SEARCHES + 4}`);
	});

	it('removes case-insensitively', () => {
		expect(removeRecent(['Forge', 'b'], 'forge')).toEqual(['b']);
	});

	it('matches by prefix and skips the typed text itself', () => {
		const list = ['forge welding', 'forge', 'knife', 'Forge build'];
		expect(matchRecent(list, 'forg', 5)).toEqual(['forge welding', 'forge', 'Forge build']);
		expect(matchRecent(list, 'forge', 1)).toEqual(['forge welding']);
		expect(matchRecent(list, '', 2)).toEqual(['forge welding', 'forge']);
	});

	it('validates stored values', () => {
		expect(isRecentList(['a'])).toBe(true);
		expect(isRecentList([1])).toBe(false);
		expect(isRecentList({})).toBe(false);
	});
});
