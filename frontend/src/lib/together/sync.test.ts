import { describe, expect, it } from 'vitest';
import {
	ClockSync,
	DriftController,
	MAX_RATE_CHANGE,
	backoffDelay,
	cleanName,
	livePosition,
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

describe('backoffDelay', () => {
	it('doubles up to 30 s with jitter on top', () => {
		expect(backoffDelay(0, () => 0)).toBe(1000);
		expect(backoffDelay(3, () => 0)).toBe(8000);
		expect(backoffDelay(10, () => 0)).toBe(30_000);
		expect(backoffDelay(0, () => 1)).toBe(1200);
	});
});

describe('ClockSync', () => {
	it('averages both directions of a round trip', () => {
		const clock = new ClockSync();
		expect(clock.offset(7)).toBe(7);
		expect(clock.rtt).toBeNull();
		// Server 5000 ms ahead, 100 ms each way.
		clock.sample(1000, 1200, 6100);
		expect(clock.offset()).toBe(5000);
		expect(clock.rtt).toBe(200);
	});

	it('keeps the median of the fastest recent round trips', () => {
		const clock = new ClockSync();
		const offset = 5000;
		// A slow reply: the request took 10 ms, the reply 490 ms, so the estimate is 240 ms off.
		clock.sample(0, 500, 10 + offset);
		for (const [i, oneWay] of [20, 5, 6, 4, 30, 8].entries()) {
			const sent = 1000 * (i + 1);
			// Slightly asymmetric: replies take 1 ms longer.
			clock.sample(sent, sent + 2 * oneWay + 1, sent + oneWay + offset);
		}
		expect(Math.abs(clock.offset() - offset)).toBeLessThanOrEqual(0.5);
		expect(clock.rtt).toBe(9);
		expect(clock.samples).toBe(7);
	});

	it('forgets old samples and can start over', () => {
		const clock = new ClockSync();
		clock.sample(0, 2, 101);
		for (let i = 1; i <= 10; i++) clock.sample(i * 1000, i * 1000 + 20, i * 1000 + 10 + 300);
		expect(clock.samples).toBe(10);
		expect(clock.offset()).toBe(300);
		clock.reset();
		expect(clock.samples).toBe(0);
		expect(clock.offset(-4)).toBe(-4);
	});
});

describe('DriftController', () => {
	it('leaves small drift alone and slows down or speeds up beyond it', () => {
		const control = new DriftController();
		expect(control.update(0.02, 0)).toBe(1);
		const ahead = new DriftController();
		expect(ahead.update(0.08, 0)).toBe(0.96);
		const behind = new DriftController();
		expect(behind.update(-0.08, 0)).toBe(1.04);
	});

	it('never changes the speed by more than the limit', () => {
		const control = new DriftController();
		expect(control.update(0.9, 0)).toBe(1 - MAX_RATE_CHANGE);
		control.reset();
		expect(control.update(-0.9, 0)).toBe(1 + MAX_RATE_CHANGE);
	});

	it('seeks for large drift', () => {
		const control = new DriftController();
		expect(control.update(1.5, 0)).toBe('seek');
		expect(control.update(-3, 0)).toBe('seek');
		expect(control.rate).toBe(1);
	});

	it('converges without oscillating', () => {
		const control = new DriftController();
		const tick = 0.25;
		let drift = -0.4;
		let rate = 1;
		const rates: number[] = [];
		for (let t = 0; t < 30; t += tick) {
			const action = control.update(drift, t * 1000);
			expect(action).not.toBe('seek');
			rate = action as number;
			rates.push(rate);
			drift += (rate - 1) * tick;
		}
		expect(Math.abs(drift)).toBeLessThan(0.03);
		expect(rate).toBe(1);
		// Catching up from behind never overshoots into slowing down.
		expect(rates.every((r) => r >= 1)).toBe(true);
	});

	it('stops at the inner edge of the deadband', () => {
		const control = new DriftController();
		expect(control.update(0.05, 0)).toBeLessThan(1);
		expect(control.update(0.02, 250)).toBeLessThan(1);
		for (let t = 500; t <= 1500; t += 250) control.update(0, t);
		expect(control.rate).toBe(1);
		expect(control.update(0.02, 1750)).toBe(1);
	});

	it('learns how far ahead to seek from where seeks land', () => {
		const control = new DriftController();
		const lead = control.lead;
		control.seeked(0);
		// Landed 400 ms behind the room: aim further ahead next time.
		control.update(-0.4, 2000);
		expect(control.lead).toBeCloseTo(lead + 0.4);
		control.seeked(10_000);
		control.update(0.1, 11_000);
		expect(control.lead).toBeCloseTo(lead + 0.3);
		// Only the first measurement after a seek counts, and only soon after it.
		control.update(-0.5, 12_000);
		expect(control.lead).toBeCloseTo(lead + 0.3);
		control.seeked(20_000);
		control.update(-0.5, 60_000);
		expect(control.lead).toBeCloseTo(lead + 0.3);
		control.seeked(70_000);
		control.update(5, 71_000);
		expect(control.lead).toBe(0);
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
