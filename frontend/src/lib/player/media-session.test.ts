import { describe, expect, it, vi } from 'vitest';
import { positionState, sessionHandlers } from './media-session';

const details = (d: Partial<MediaSessionActionDetails>): MediaSessionActionDetails => ({
	action: 'seekto',
	...d
});

describe('sessionHandlers', () => {
	it('registers nothing without actions', () => {
		expect(Object.values(sessionHandlers(null)).every((h) => h === null)).toBe(true);
	});

	it('seeks relative to the position and clamps at the start', () => {
		const seekTo = vi.fn();
		const h = sessionHandlers({ play() {}, pause() {}, seekTo, position: () => 4 });
		h.seekbackward!(details({ action: 'seekbackward' }));
		expect(seekTo).toHaveBeenLastCalledWith(0);
		h.seekforward!(details({ action: 'seekforward', seekOffset: 30 }));
		expect(seekTo).toHaveBeenLastCalledWith(34);
		h.seekto!(details({ seekTime: 90 }));
		expect(seekTo).toHaveBeenLastCalledWith(90);
	});

	it('leaves out seeking, next and previous when not offered', () => {
		const h = sessionHandlers({ play() {}, pause() {} });
		expect(h.play).not.toBeNull();
		expect(h.pause).not.toBeNull();
		expect(h.seekto).toBeNull();
		expect(h.seekforward).toBeNull();
		expect(h.nexttrack).toBeNull();
		expect(h.previoustrack).toBeNull();
	});

	it('passes next and previous through', () => {
		const next = vi.fn();
		const previous = vi.fn();
		const h = sessionHandlers({ play() {}, pause() {}, next, previous });
		h.nexttrack!(details({ action: 'nexttrack' }));
		h.previoustrack!(details({ action: 'previoustrack' }));
		expect(next).toHaveBeenCalledOnce();
		expect(previous).toHaveBeenCalledOnce();
	});
});

describe('positionState', () => {
	it('needs a finite duration', () => {
		expect(positionState(5, NaN, 1)).toBeNull();
		expect(positionState(5, Infinity, 1)).toBeNull();
		expect(positionState(5, 0, 1)).toBeNull();
	});

	it('keeps the position within the duration and the rate positive', () => {
		expect(positionState(130, 120, 1.5)).toEqual({
			duration: 120,
			position: 120,
			playbackRate: 1.5
		});
		expect(positionState(-1, 120, 0)).toEqual({ duration: 120, position: 0, playbackRate: 1 });
	});
});
