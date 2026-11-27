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

/**
 * Keeps a playing player on the room's position: small drift is made up by playing slightly
 * faster or slower, large drift by seeking ahead of the room by how long seeks took before.
 */
export class DriftController {
	/** Seconds to seek ahead of the room, learned from where earlier seeks landed. */
	lead = INITIAL_SEEK_LEAD;
	rate = 1;
	#drift: number | null = null;
	#correcting = false;
	#seekedAt: number | null = null;

	/** Call on a jump to the room's position plus `lead`, while playing. */
	seeked(now: number) {
		this.#seekedAt = now;
		this.reset();
	}

	/** Forgets the drift, e.g. while buffering or paused. */
	reset() {
		this.#drift = null;
		this.#correcting = false;
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
		if (Math.abs(drift) > PLAYING_DRIFT) {
			this.reset();
			return 'seek';
		}
		this.#drift = this.#drift === null ? drift : this.#drift + SMOOTHING * (drift - this.#drift);
		const size = Math.abs(this.#drift);
		this.#correcting = size > DEADBAND[this.#correcting ? 1 : 0];
		if (!this.#correcting) {
			this.rate = 1;
			return this.rate;
		}
		const change = Math.min(MAX_RATE_CHANGE, Math.max(-MAX_RATE_CHANGE, -GAIN * this.#drift));
		this.rate = Math.round((1 + change) / RATE_STEP) * RATE_STEP;
		this.rate = Math.round(this.rate * 1000) / 1000;
		return this.rate;
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
