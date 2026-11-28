import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MemberStatus } from './connection.svelte';
import {
	PresenceNotices,
	REJOIN_MS,
	StatusReporter,
	placeOf,
	syncHealth,
	type PresenceNotice
} from './presence';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

const clock = { now: () => Date.now() };

describe('syncHealth', () => {
	it('maps the player state and drift', () => {
		expect(syncHealth('locked', 0.02)).toBe('synced');
		expect(syncHealth('catching up', -0.4)).toBe('synced');
		expect(syncHealth('paused', 3)).toBe('behind');
		expect(syncHealth('buffering', 0)).toBe('buffering');
		expect(syncHealth('seeking', 4)).toBe('buffering');
		expect(syncHealth('blocked', null)).toBe('behind');
		expect(syncHealth('reporting', null)).toBeUndefined();
	});
});

describe('placeOf', () => {
	it('derives the place from the route', () => {
		expect(placeOf('/watch', 'abc', 'abc')).toBe('watching');
		expect(placeOf('/watch', 'xyz', 'abc')).toBe('browsing');
		expect(placeOf('/shorts/[id=videoid]', null, 'abc')).toBe('shorts');
		expect(placeOf('/results', null, 'abc')).toBe('browsing');
		expect(placeOf('/watch', 'abc', null)).toBe('browsing');
	});
});

describe('StatusReporter', () => {
	it('sends changes once they held, at most every 2 s', () => {
		const sent: MemberStatus[] = [];
		const reporter = new StatusReporter((s) => sent.push(s), clock);
		reporter.update({ sync: 'synced', place: 'watching' });
		expect(sent).toEqual([{ sync: 'synced', place: 'watching' }]);

		// A short stall is not reported.
		reporter.update({ sync: 'buffering', place: 'watching' });
		vi.advanceTimersByTime(400);
		reporter.update({ sync: 'synced', place: 'watching' });
		vi.advanceTimersByTime(5000);
		expect(sent).toHaveLength(1);

		// A long one is, once it held a second.
		reporter.update({ sync: 'buffering', place: 'watching' });
		vi.advanceTimersByTime(999);
		expect(sent).toHaveLength(1);
		vi.advanceTimersByTime(1);
		expect(sent.at(-1)).toEqual({ sync: 'buffering', place: 'watching' });

		// Places change right away, but no sooner than 2 s after the last report.
		reporter.update({ sync: 'buffering', place: 'browsing' });
		expect(sent).toHaveLength(2);
		vi.advanceTimersByTime(2000);
		expect(sent.at(-1)).toEqual({ sync: 'buffering', place: 'browsing' });
		reporter.destroy();
	});
});

describe('PresenceNotices', () => {
	const grace = { id: 'p2', name: 'Grace', is_guest: true };
	const ada = { id: 'p3', name: 'Ada', is_guest: true };
	const me = { id: 'p1', name: 'mara', is_guest: false };

	function notices() {
		const seen: (PresenceNotice | null)[] = [];
		return { seen, presence: new PresenceNotices((n) => seen.push(n), clock) };
	}

	it('skips your own join and the people already there', () => {
		const { seen, presence } = notices();
		presence.update([grace, me], 'p1');
		expect(seen).toEqual([]);
		presence.update([grace, me, ada], 'p1');
		expect(seen.at(-1)).toMatchObject({ kind: 'joined', names: ['Ada'] });
		vi.advanceTimersByTime(5000);
		expect(seen.at(-1)).toBeNull();
		presence.destroy();
	});

	it('coalesces bursts', () => {
		const { seen, presence } = notices();
		presence.update([me], 'p1');
		presence.update([me, grace], 'p1');
		vi.advanceTimersByTime(500);
		presence.update([me, grace, ada], 'p1');
		expect(seen.at(-1)).toMatchObject({ kind: 'joined', names: ['Grace', 'Ada'] });
		presence.destroy();
	});

	it('says nothing about a quick reconnect, but tells who left', () => {
		const { seen, presence } = notices();
		presence.update([me, grace], 'p1');
		presence.update([me], 'p1');
		vi.advanceTimersByTime(1500);
		presence.update([me, { ...grace, id: 'p9' }], 'p1');
		vi.advanceTimersByTime(REJOIN_MS);
		expect(seen).toEqual([]);

		presence.update([me], 'p1');
		vi.advanceTimersByTime(REJOIN_MS - 1);
		expect(seen).toEqual([]);
		vi.advanceTimersByTime(1);
		expect(seen.at(-1)).toMatchObject({ kind: 'left', names: ['Grace'] });
		presence.destroy();
	});

	it('starts over when this tab reconnects', () => {
		const { seen, presence } = notices();
		presence.update([me, grace], 'p1');
		presence.update([{ ...me, id: 'p5' }, grace, ada], 'p5');
		expect(seen).toEqual([]);
		presence.destroy();
	});
});
