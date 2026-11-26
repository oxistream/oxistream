import { describe, expect, it, vi } from 'vitest';
import type { VideoSummary } from '#lib/video.js';
import { createWheelPager, dragOffset, isVerticalDrag, keyStep, releaseStep } from './gestures';
import {
	buildQueue,
	discoveryQueue,
	markShortSeen,
	needsMore,
	seenShorts,
	rememberShortsList,
	shortSlots,
	shortStub,
	takeShortsList
} from './queue';

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

	it('stacks the neighbours around the current short and preloads only the next one', () => {
		const [a, b, c] = ['a', 'b', 'c'].map(short);
		const shape = (slots: ReturnType<typeof shortSlots>) =>
			slots.map((s) => [s.short.id, s.offset, s.preload]);
		expect(shape(shortSlots(b, a, c))).toEqual([
			['a', -1, false],
			['b', 0, true],
			['c', 1, true]
		]);
		expect(shape(shortSlots(a, undefined, undefined))).toEqual([['a', 0, true]]);
	});

	it('skips discovered shorts shown earlier in the session', () => {
		markShortSeen('a');
		markShortSeen('a');
		const seen = seenShorts();
		expect([...seen]).toEqual(['a']);
		const items = ['a', 'b', 'b', 'c'].map(short);
		expect(discoveryQueue(items, seen).map((s) => s.id)).toEqual(['b', 'c']);
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

	it('tells vertical drags from taps and sideways moves', () => {
		expect(isVerticalDrag(2, 4)).toBe(false);
		expect(isVerticalDrag(3, -20)).toBe(true);
		expect(isVerticalDrag(40, 20)).toBe(false);
	});

	it('resists dragging past the ends', () => {
		expect(dragOffset(-100, () => true)).toBe(-100);
		expect(dragOffset(100, (step) => step === 1)).toBe(25);
	});

	it('snaps released drags that went far or fast, springs back otherwise', () => {
		expect(releaseStep(-250, 0, 800)).toBe(1);
		expect(releaseStep(220, 0, 800)).toBe(-1);
		expect(releaseStep(-60, -0.8, 800)).toBe(1);
		expect(releaseStep(-60, -0.1, 800)).toBeNull();
		expect(releaseStep(-10, -2, 800)).toBeNull();
		expect(releaseStep(-60, 0.8, 800)).toBeNull();
	});
});
