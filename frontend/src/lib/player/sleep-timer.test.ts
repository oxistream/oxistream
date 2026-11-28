import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SleepTimer, isSleepMode } from './sleep-timer.svelte';

describe('SleepTimer', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});
	afterEach(() => {
		vi.useRealTimers();
	});

	it('pauses after the chosen minutes and turns off', () => {
		const expire = vi.fn();
		const timer = new SleepTimer(expire);
		timer.set('15');
		expect(timer.remaining()).toBe(15);
		vi.advanceTimersByTime(14 * 60_000);
		expect(expire).not.toHaveBeenCalled();
		expect(timer.remaining()).toBe(1);
		vi.advanceTimersByTime(60_000);
		expect(expire).toHaveBeenCalledOnce();
		expect(timer.mode).toBe('off');
		expect(timer.remaining()).toBeNull();
	});

	it('restarts the countdown when changed', () => {
		const expire = vi.fn();
		const timer = new SleepTimer(expire);
		timer.set('30');
		vi.advanceTimersByTime(20 * 60_000);
		timer.set('15');
		vi.advanceTimersByTime(14 * 60_000);
		expect(expire).not.toHaveBeenCalled();
		vi.advanceTimersByTime(60_000);
		expect(expire).toHaveBeenCalledOnce();
	});

	it('does nothing once turned off', () => {
		const expire = vi.fn();
		const timer = new SleepTimer(expire);
		timer.set('45');
		timer.set('off');
		vi.advanceTimersByTime(60 * 60_000);
		expect(expire).not.toHaveBeenCalled();
		expect(timer.ended()).toBe(false);
	});

	it('stops at the end of the video in end mode', () => {
		const expire = vi.fn();
		const timer = new SleepTimer(expire);
		timer.set('end');
		expect(timer.remaining()).toBeNull();
		vi.advanceTimersByTime(120 * 60_000);
		expect(expire).not.toHaveBeenCalled();
		expect(timer.ended()).toBe(true);
		expect(expire).toHaveBeenCalledOnce();
		expect(timer.mode).toBe('off');
		expect(timer.ended()).toBe(false);
	});

	it('lets videos go on in minute modes', () => {
		const timer = new SleepTimer(() => {});
		timer.set('60');
		expect(timer.ended()).toBe(false);
		expect(timer.mode).toBe('60');
	});
});

describe('isSleepMode', () => {
	it('accepts only the offered modes', () => {
		expect(isSleepMode('30')).toBe(true);
		expect(isSleepMode('end')).toBe(true);
		expect(isSleepMode('20')).toBe(false);
		expect(isSleepMode(30)).toBe(false);
	});
});
