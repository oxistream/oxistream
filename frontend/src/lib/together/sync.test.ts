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

/**
 * Runs the controller against a player that freezes for `freeze` seconds on every rate change,
 * as WebKit does, ticking every 250 ms like the room player.
 */
function simulate(control: DriftController, drift: number, seconds: number, freeze: number) {
	const tick = 0.25;
	let rate = 1;
	let frozen = 0;
	let changes = 0;
	const drifts: number[] = [];
	for (let t = 0; t < seconds; t += tick) {
		const action = control.update(drift, t * 1000);
		expect(action).not.toBe('seek');
		if (action !== rate) {
			rate = action as number;
			changes++;
			frozen += freeze;
		}
		const still = Math.min(tick, frozen);
		frozen -= still;
		drift += (rate - 1) * (tick - still) - still;
		drifts.push(drift);
	}
	return { drift, changes, drifts, rate };
}

describe('DriftController with costly rate changes', () => {
	it('learns what a rate change costs from how drift moved after it', () => {
		const control = new DriftController();
		expect(control.costly).toBe(false);
		simulate(control, -0.4, 60, 0.3);
		expect(control.rateCost).toBeGreaterThan(0.2);
		expect(control.costly).toBe(true);
	});

	it('is not fooled by a stall that happens once after a change', () => {
		const control = new DriftController();
		expect(control.update(-0.08, 0)).toBeGreaterThan(1);
		// Played on at the new rate, but lost 750 ms to a hiccup.
		control.update(-0.08 - 0.75, 1500);
		expect(control.costly).toBe(false);
		const run = simulate(control, -0.4, 30, 0);
		expect(control.costly).toBe(false);
		expect(Math.abs(run.drift)).toBeLessThan(0.03);
	});

	it('stays fine-grained when rate changes are free', () => {
		const control = new DriftController();
		const run = simulate(control, -0.4, 30, 0);
		expect(control.rateCost).toBe(0);
		expect(Math.abs(run.drift)).toBeLessThan(0.03);
	});

	it('makes up drift in one phase of two rate changes, freezes included', () => {
		for (const start of [-0.6, -0.15, 0.15, 0.5, 0.9]) {
			const control = new DriftController(0.3);
			const run = simulate(control, start, 60, 0.3);
			expect(run.changes).toBe(2);
			expect(run.rate).toBe(1);
			expect(Math.abs(run.drift)).toBeLessThan(0.03);
		}
	});

	it('leaves drift inside the wider deadband alone', () => {
		const control = new DriftController(0.3);
		const run = simulate(control, 0.06, 30, 0.3);
		expect(run.changes).toBe(0);
	});

	it('settles with few rate changes before the cost is known', () => {
		const control = new DriftController();
		const run = simulate(control, -0.5, 120, 0.3);
		expect(run.changes).toBeLessThanOrEqual(6);
		expect(Math.abs(run.drift)).toBeLessThan(0.05);
		// Once settled, nothing changes any more.
		expect(run.drifts.slice(-200).every((d) => Math.abs(d - run.drift) < 0.001)).toBe(true);
	});

	it('remembers the cost across videos', () => {
		expect(new DriftController(0.25).costly).toBe(true);
		expect(new DriftController(0.02).costly).toBe(false);
		expect(new DriftController(9).rateCost).toBeLessThanOrEqual(1.5);
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
