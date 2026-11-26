import { readJson, writeJson } from '#lib/components/player/storage.js';
import { isVideoId, parseYouTubeUrl } from '#lib/youtube.js';

/** Beyond this many seconds apart while playing, the player jumps to the room's position. */
export const PLAYING_DRIFT = 1.5;
/** While paused, everyone should show the same frame. */
export const PAUSED_DRIFT = 0.25;
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

/** Whether a player at `local` seconds should jump to `target`. */
export function needsSeek(local: number, target: number, playing: boolean): boolean {
	return Math.abs(local - target) > (playing ? PLAYING_DRIFT : PAUSED_DRIFT);
}

/** Reconnect delay: doubling from 1 s up to 30 s, with up to 20 % jitter. */
export function backoffDelay(attempt: number, random = Math.random): number {
	const base = Math.min(30_000, 1000 * 2 ** Math.max(0, attempt));
	return Math.round(base * (1 + 0.2 * random()));
}

/**
 * Server clock offset from ping round trips: the sample with the shortest round trip is the
 * most accurate, assuming the reply took half of it.
 */
export class ClockSync {
	#best: { rtt: number; offset: number } | undefined;

	/** `sent` and `received` are local times; `serverTime` comes from the pong. */
	sample(sent: number, received: number, serverTime: number) {
		const rtt = Math.max(0, received - sent);
		const offset = serverTime - (sent + rtt / 2);
		if (!this.#best || rtt <= this.#best.rtt) this.#best = { rtt, offset };
	}

	/** Without a ping yet, `fallback` (e.g. from the welcome message) is used. */
	offset(fallback = 0): number {
		return this.#best?.offset ?? fallback;
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
