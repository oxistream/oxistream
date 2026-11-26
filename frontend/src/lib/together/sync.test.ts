import { describe, expect, it } from 'vitest';
import {
	ClockSync,
	backoffDelay,
	cleanName,
	livePosition,
	needsSeek,
	videoIdFromInput
} from './sync';

describe('livePosition', () => {
	it('advances with the server clock only while playing', () => {
		const at = 1_000_000;
		expect(livePosition({ playing: true, position: 10, positionAt: at }, at + 2500)).toBe(12.5);
		expect(livePosition({ playing: false, position: 10, positionAt: at }, at + 2500)).toBe(10);
		expect(livePosition({ playing: true, position: 10, positionAt: at }, at - 500)).toBe(10);
	});
});

describe('needsSeek', () => {
	it('tolerates more drift while playing than while paused', () => {
		expect(needsSeek(10, 11, true)).toBe(false);
		expect(needsSeek(10, 12, true)).toBe(true);
		expect(needsSeek(10, 10.2, false)).toBe(false);
		expect(needsSeek(10, 10.5, false)).toBe(true);
	});
});

describe('backoffDelay', () => {
	it('doubles up to 30 s with jitter on top', () => {
		expect(backoffDelay(0, () => 0)).toBe(1000);
		expect(backoffDelay(3, () => 0)).toBe(8000);
		expect(backoffDelay(10, () => 0)).toBe(30_000);
		expect(backoffDelay(0, () => 1)).toBe(1200);
	});
});

describe('ClockSync', () => {
	it('keeps the offset of the fastest round trip', () => {
		const clock = new ClockSync();
		expect(clock.offset(7)).toBe(7);
		clock.sample(1000, 1200, 6100);
		expect(clock.offset()).toBe(5000);
		clock.sample(2000, 2020, 7020);
		expect(clock.offset()).toBe(5010);
		clock.sample(3000, 3500, 9000);
		expect(clock.offset()).toBe(5010);
	});
});

describe('videoIdFromInput', () => {
	it('accepts IDs and the usual YouTube links', () => {
		const id = 'dQw4w9WgXcQ';
		expect(videoIdFromInput(` ${id} `)).toBe(id);
		expect(videoIdFromInput(`https://www.youtube.com/watch?v=${id}&t=42`)).toBe(id);
		expect(videoIdFromInput(`youtu.be/${id}`)).toBe(id);
		expect(videoIdFromInput(`https://youtube.com/shorts/${id}`)).toBe(id);
		expect(videoIdFromInput(`https://m.youtube.com/embed/${id}`)).toBe(id);
	});

	it('rejects other input', () => {
		expect(videoIdFromInput('forging a bottle opener')).toBeNull();
		expect(videoIdFromInput('https://example.org/watch?v=dQw4w9WgXcQ')).toBeNull();
		expect(videoIdFromInput('https://www.youtube.com/@handle')).toBeNull();
	});
});

describe('cleanName', () => {
	it('trims, drops control characters and caps the length', () => {
		expect(cleanName('  Ada\u0007 ')).toBe('Ada');
		expect(cleanName('x'.repeat(40))).toHaveLength(32);
	});
});
