import { describe, expect, it, vi } from 'vitest';
import type { VideoSummary } from '#lib/video.js';
import { createWheelPager, keyStep, swipeStep } from './gestures';
import { createLoopingReporter } from './progress';
import { buildQueue, needsMore, rememberShortsList, shortStub, takeShortsList } from './queue';

const short = (id: string): VideoSummary => ({
	id,
	title: `Short ${id}`,
	channel: { id: 'UCx', name: 'X' }
});

describe('shorts queue', () => {
	it('continues through the list a short was opened from, once', () => {
		const list = [short('a'), short('b'), short('a'), short('c')];
		rememberShortsList(list);
		expect(takeShortsList('b')?.map((s) => s.id)).toEqual(['a', 'b', 'c']);
		expect(takeShortsList('b')).toBeUndefined();
	});

	it('ignores a remembered list without the opened short', () => {
		rememberShortsList([short('a')]);
		expect(takeShortsList('z')).toBeUndefined();
	});

	it('falls back to the channel shorts, keeping the place of the opened one', () => {
		const channel = [short('a'), short('b'), short('c')];
		expect(buildQueue('b', undefined, channel).map((s) => s.id)).toEqual(['a', 'b', 'c']);
		expect(buildQueue('z', undefined, channel).map((s) => s.id)).toEqual(['z', 'a', 'b', 'c']);
		expect(buildQueue('z', undefined, [])).toEqual([shortStub('z')]);
		const context = [short('x'), short('z')];
		expect(buildQueue('z', context, channel)).toBe(context);
	});

	it('asks for more near the end', () => {
		const queue = ['a', 'b', 'c', 'd', 'e'].map(short);
		expect(needsMore(queue, 0)).toBe(false);
		expect(needsMore(queue, 2)).toBe(true);
		expect(needsMore(queue, -1)).toBe(false);
	});
});

describe('shorts gestures', () => {
	const key = (k: string, extra: Partial<KeyboardEvent> = {}) => ({
		key: k,
		ctrlKey: false,
		metaKey: false,
		altKey: false,
		shiftKey: false,
		target: null,
		...extra
	});

	it('maps arrows and j/k to steps', () => {
		expect(keyStep(key('ArrowDown'))).toBe(1);
		expect(keyStep(key('j'))).toBe(1);
		expect(keyStep(key('ArrowUp'))).toBe(-1);
		expect(keyStep(key('k'))).toBe(-1);
		expect(keyStep(key(' '))).toBeNull();
		expect(keyStep(key('j', { ctrlKey: true }))).toBeNull();
	});

	it('leaves keys to form fields and sliders', () => {
		const target = { closest: (selector: string) => (selector.includes('slider') ? {} : null) };
		expect(keyStep(key('ArrowDown', { target: target as unknown as EventTarget }))).toBeNull();
	});

	it('steps once per wheel gesture', () => {
		const onstep = vi.fn();
		const wheel = createWheelPager(onstep);
		for (let t = 0; t < 600; t += 16) wheel({ deltaY: 30, deltaMode: 0, timeStamp: t });
		expect(onstep).toHaveBeenCalledTimes(1);
		wheel({ deltaY: -3, deltaMode: 1, timeStamp: 1000 });
		expect(onstep).toHaveBeenLastCalledWith(-1);
		expect(onstep).toHaveBeenCalledTimes(2);
	});

	it('ignores small wheel nudges', () => {
		const onstep = vi.fn();
		const wheel = createWheelPager(onstep);
		wheel({ deltaY: 10, deltaMode: 0, timeStamp: 0 });
		wheel({ deltaY: 10, deltaMode: 0, timeStamp: 500 });
		expect(onstep).not.toHaveBeenCalled();
	});

	it('reads vertical swipes', () => {
		expect(swipeStep({ x: 0, y: 400 }, { x: 10, y: 200 })).toBe(1);
		expect(swipeStep({ x: 0, y: 200 }, { x: 0, y: 400 })).toBe(-1);
		expect(swipeStep({ x: 0, y: 200 }, { x: 0, y: 230 })).toBeNull();
		expect(swipeStep({ x: 0, y: 200 }, { x: 200, y: 300 })).toBeNull();
	});
});

describe('looping progress', () => {
	it('stays finished once a short got far enough', () => {
		const report = vi.fn();
		const reporter = createLoopingReporter(report);
		reporter.update(28, 30);
		reporter.report({ position: 2, duration: 30, finished: false, reason: 'interval' });
		expect(report).toHaveBeenCalledWith({
			position: 2,
			duration: 30,
			finished: true,
			reason: 'interval'
		});
	});

	it('reports unfinished shorts as they are', () => {
		const report = vi.fn();
		const reporter = createLoopingReporter(report);
		reporter.update(5, 30);
		reporter.report({ position: 5, duration: 30, finished: false, reason: 'pause' });
		expect(report).toHaveBeenCalledWith(expect.objectContaining({ finished: false }));
	});
});
