import { readJson, writeJson } from '#lib/components/player/storage.js';
import { isVideoId, parseYouTubeUrl } from '#lib/youtube.js';

/** Beyond this many seconds apart while playing, the player jumps instead of catching up. */
export const PLAYING_DRIFT = 1;
/** While paused, everyone should show the same frame. */
export const PAUSED_DRIFT = 0.04;
/** Catching up changes the speed by at most this much, which nobody hears with pitch kept. */
export const MAX_RATE_CHANGE = 0.05;
/** A seek by the user moves at least this far from where the room is. */
export const SEEK_THRESHOLD = 1;
export const MAX_NAME_LENGTH = 32;

const NAME_KEY = 'oxistream:together-name';
const GUEST_KEY = 'oxistream:together-guest';
const RATE_COST_KEY = 'oxistream:together-rate-cost';

export type Playback = {
	playing: boolean;
	/** Seconds at `positionAt`. */
	position: number;
	/** Server time in ms since the epoch. */
	positionAt: number;
};

/** Where the room is now, in seconds, given the server's clock. */
export function livePosition(playback: Playback, serverNow: number): number {
	if (!playback.playing) return playback.position;
	return playback.position + Math.max(0, serverNow - playback.positionAt) / 1000;
}

/** A monotonic local clock in ms, roughly since the epoch. */
export function localNow(): number {
	return performance.timeOrigin + performance.now();
}

/** Reconnect delay: doubling from 1 s up to 30 s, with up to 20 % jitter. */
export function backoffDelay(attempt: number, random = Math.random): number {
	const base = Math.min(30_000, 1000 * 2 ** Math.max(0, attempt));
	return Math.round(base * (1 + 0.2 * random()));
}

const CLOCK_SAMPLES = 10;
const CLOCK_BEST = 3;

/**
 * Server clock offset, NTP style: of the last few pings, the ones with the shortest round trip
 * are the most accurate, assuming the reply took half of it; the median of those is kept.
 */
export class ClockSync {
	#samples: { rtt: number; offset: number }[] = [];

	/** `sent` and `received` are local times; `serverTime` comes from the pong. */
	sample(sent: number, received: number, serverTime: number) {
		const rtt = Math.max(0, received - sent);
		const offset = (serverTime - sent + (serverTime - received)) / 2;
		this.#samples.push({ rtt, offset });
		if (this.#samples.length > CLOCK_SAMPLES) this.#samples.shift();
	}

	/** Forgets all samples, e.g. after reconnecting to another server. */
	reset() {
		this.#samples = [];
	}

	get samples(): number {
		return this.#samples.length;
	}

	/** The shortest round trip in ms, an upper bound of the offset's error. */
	get rtt(): number | null {
		return this.#samples.length ? Math.min(...this.#samples.map((s) => s.rtt)) : null;
	}

	/** Without a ping yet, `fallback` (e.g. from the welcome message) is used. */
	offset(fallback = 0): number {
		if (!this.#samples.length) return fallback;
		const best = [...this.#samples].sort((a, b) => a.rtt - b.rtt).slice(0, CLOCK_BEST);
		const offsets = best.map((s) => s.offset).sort((a, b) => a - b);
		return offsets[Math.floor((offsets.length - 1) / 2)];
	}
}

/** Starts catching up beyond this drift, in seconds, and stops below the second. */
const DEADBAND = [0.03, 0.01] as const;
/** Rate change per second of drift: half of it is made up every second. */
const GAIN = 0.5;
const RATE_STEP = 0.005;
const SMOOTHING = 0.35;
const INITIAL_SEEK_LEAD = 0.3;
const MAX_SEEK_LEAD = 3;
/** A seek's landing only teaches the lead if playback resumed this soon after it, in ms. */
const LANDING_WINDOW = 8000;
/** Seconds playback loses per rate change above which catching up uses few, long phases. */
const COSTLY_RATE_CHANGE = 0.08;
const MAX_RATE_COST = 1.5;
/** A rate change's cost is measured this long after it, in ms, if the rate held. */
const COST_WINDOW = 1500;
/** With costly rate changes, a phase starts beyond this drift and lasts at least this long. */
const COSTLY_DEADBAND = 0.08;
const MIN_PHASE_MS = 1000;

/**
 * Keeps a playing player on the room's position: small drift is made up by playing slightly
 * faster or slower, large drift by seeking ahead of the room by how long seeks took before.
 *
 * Some engines (WebKit) freeze playback for a moment on every rate change. That loss is learned
 * from how drift moved after earlier changes; when it is large, drift is made up in rare phases
 * at the full rate change, timed so the freezes at both ends are accounted for.
 */
export class DriftController {
	/** Seconds to seek ahead of the room, learned from where earlier seeks landed. */
	lead = INITIAL_SEEK_LEAD;
	rate = 1;
	/** Seconds of playback lost per rate change. */
	rateCost: number;
	#drift: number | null = null;
	#correcting = false;
	#seekedAt: number | null = null;
	#change: { at: number; drift: number; rate: number } | null = null;
	/** When the current or the last phase started or ended. */
	#phaseAt = -Infinity;
	#lastAt: number | null = null;

	constructor(rateCost = 0) {
		this.rateCost = Math.min(MAX_RATE_COST, Math.max(0, rateCost));
	}

	get costly(): boolean {
		return this.rateCost > COSTLY_RATE_CHANGE;
	}

	/** Call on a jump to the room's position plus `lead`, while playing. */
	seeked(now: number) {
		this.#seekedAt = now;
		this.reset();
	}

	/** Forgets the drift, e.g. while buffering or paused. */
	reset() {
		this.#drift = null;
		this.#correcting = false;
		this.#change = null;
		this.#lastAt = null;
		this.rate = 1;
	}

	/**
	 * `drift` is the local position minus the room's, in seconds, measured while playback runs.
	 * Returns `seek` to jump to the room's position plus `lead`, else the playback rate.
	 */
	update(drift: number, now: number): 'seek' | number {
		if (this.#seekedAt !== null) {
			if (now - this.#seekedAt < LANDING_WINDOW) {
				this.lead = Math.min(MAX_SEEK_LEAD, Math.max(0, this.lead - drift));
			}
			this.#seekedAt = null;
		}
		this.#learnCost(drift, now);
		const tick = this.#lastAt === null ? 0 : (now - this.#lastAt) / 1000;
		this.#lastAt = now;
		if (Math.abs(drift) > PLAYING_DRIFT + (this.costly ? this.rateCost : 0)) {
			this.reset();
			return 'seek';
		}
		this.#drift = this.#drift === null ? drift : this.#drift + SMOOTHING * (drift - this.#drift);
		const previous = this.rate;
		this.rate = this.costly ? this.#phase(drift, now, tick) : this.#steer();
		if (this.rate !== previous) this.#change = { at: now, drift, rate: this.rate };
		return this.rate;
	}

	#steer(): number {
		const drift = this.#drift!;
		this.#correcting = Math.abs(drift) > DEADBAND[this.#correcting ? 1 : 0];
		if (!this.#correcting) return 1;
		const change = Math.min(MAX_RATE_CHANGE, Math.max(-MAX_RATE_CHANGE, -GAIN * drift));
		return Math.round(Math.round((1 + change) / RATE_STEP) * RATE_STEP * 1000) / 1000;
	}

	/**
	 * Both rate changes of a phase lose `rateCost`, so it starts toward a drift of `+rateCost`
	 * and ends there, and the freeze of going back to normal speed lands it on the room.
	 */
	#phase(drift: number, now: number, tick: number): number {
		const cost = this.rateCost;
		if (this.rate === 1) {
			// The freeze at the end of the last phase is still to come or just happened.
			if (now - this.#phaseAt < COST_WINDOW) {
				this.#drift = drift;
				return 1;
			}
			if (Math.abs(this.#drift!) <= COSTLY_DEADBAND) return 1;
			this.#phaseAt = now;
			return drift - cost < cost ? 1 + MAX_RATE_CHANGE : 1 - MAX_RATE_CHANGE;
		}
		if (now - this.#phaseAt < MIN_PHASE_MS) return this.rate;
		const direction = Math.sign(this.rate - 1);
		const step = Math.abs(this.rate - 1) * tick;
		if (direction * (drift - cost) < -step / 2) return this.rate;
		this.#phaseAt = now;
		return 1;
	}

	/** Drift should have moved with the new rate since a change; what is missing was lost. */
	#learnCost(drift: number, now: number) {
		const change = this.#change;
		if (!change || now - change.at < COST_WINDOW) return;
		this.#change = null;
		const expected = change.drift + ((change.rate - 1) * (now - change.at)) / 1000;
		const lost = Math.min(MAX_RATE_COST, Math.max(0, expected - drift));
		this.rateCost += 0.5 * (lost - this.rateCost);
	}
}

/** A video ID from a pasted ID or YouTube link; `null` for anything else. */
export function videoIdFromInput(input: string): string | null {
	const value = input.trim();
	if (isVideoId(value)) return value;
	const url = parseYouTubeUrl(value);
	if (!url) return null;
	const host = url.hostname.toLowerCase();
	const candidate = host.endsWith('youtu.be')
		? url.pathname.slice(1)
		: (url.searchParams.get('v') ??
			/^\/(?:shorts|embed|live|v)\/([^/]+)/.exec(url.pathname)?.[1] ??
			'');
	return isVideoId(candidate) ? candidate : null;
}

export function cleanName(name: string): string {
	return name
		.replace(/\p{Cc}/gu, '')
		.trim()
		.slice(0, MAX_NAME_LENGTH);
}

/** What a rate change cost in the last room, so the next video starts out right. */
export function savedRateCost(): number {
	return (
		readJson(RATE_COST_KEY, (v): v is number => typeof v === 'number' && Number.isFinite(v)) ?? 0
	);
}

export function saveRateCost(cost: number) {
	writeJson(RATE_COST_KEY, Math.round(cost * 1000) / 1000);
}

export function savedGuestName(): string {
	return readJson(NAME_KEY, (v): v is string => typeof v === 'string') ?? '';
}

export function saveGuestName(name: string) {
	writeJson(NAME_KEY, name);
}

/** A random key kept per browser, so guests stay the same person across reconnects. */
export function guestKey(): string {
	const saved = readJson(GUEST_KEY, (v): v is string => typeof v === 'string' && v.length >= 16);
	if (saved) return saved;
	const bytes = crypto.getRandomValues(new Uint8Array(24));
	const key = btoa(String.fromCharCode(...bytes))
		.replaceAll('+', '-')
		.replaceAll('/', '_')
		.replace(/=+$/, '');
	writeJson(GUEST_KEY, key);
	return key;
}
