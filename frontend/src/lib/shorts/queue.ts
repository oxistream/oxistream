import { uniqueBy } from '#lib/api/mapping.js';
import type { VideoSummary } from '#lib/video.js';

let remembered: VideoSummary[] | undefined;

/** Keeps the list a short was opened from, so the shorts player continues through it. */
export function rememberShortsList(shorts: VideoSummary[]) {
	remembered = shorts;
}

/** The remembered list if it contains `id`, once; a reload or another entry point starts fresh. */
export function takeShortsList(id: string): VideoSummary[] | undefined {
	const list = remembered;
	remembered = undefined;
	return list?.some((short) => short.id === id) ? uniqueBy(list, (short) => short.id) : undefined;
}

/** A placeholder until the short's details are loaded. */
export function shortStub(id: string): VideoSummary {
	return { id, title: '', channel: { id: '', name: '' } };
}

/**
 * The shorts to swipe through: the list the short was opened from, else the channel's shorts.
 * Within the channel's list the short keeps its place; otherwise it comes first.
 */
export function buildQueue(
	openedId: string,
	context: VideoSummary[] | undefined,
	channelShorts: VideoSummary[]
): VideoSummary[] {
	if (context) return context;
	const unique = uniqueBy(channelShorts, (short) => short.id);
	if (unique.some((short) => short.id === openedId)) return unique;
	return [shortStub(openedId), ...unique];
}

const SEEN_KEY = 'oxistream:shorts-seen';
const SEEN_LIMIT = 500;
let seen: string[] | undefined;

function seenList(): string[] {
	if (seen) return seen;
	try {
		const stored: unknown = JSON.parse(sessionStorage.getItem(SEEN_KEY) ?? '[]');
		seen = Array.isArray(stored) ? stored.filter((id) => typeof id === 'string') : [];
	} catch {
		seen = [];
	}
	return seen;
}

/** Shorts shown so far in this browser session. */
export function seenShorts(): Set<string> {
	return new Set(seenList());
}

export function markShortSeen(id: string) {
	const list = seenList();
	if (list.includes(id)) return;
	list.push(id);
	if (list.length > SEEN_LIMIT) list.splice(0, list.length - SEEN_LIMIT);
	try {
		sessionStorage.setItem(SEEN_KEY, JSON.stringify(list));
	} catch {
		// Without storage the list lasts until the page is reloaded.
	}
}

/** Discovered shorts without repeats and without those shown before the viewer opened. */
export function discoveryQueue(items: VideoSummary[], seenBefore: Set<string>): VideoSummary[] {
	return uniqueBy(items, (short) => short.id).filter((short) => !seenBefore.has(short.id));
}

/** Ask for more shorts once the viewer gets this close to the end of the queue. */
export const LOAD_AHEAD = 3;

export function needsMore(queue: VideoSummary[], index: number): boolean {
	return index >= 0 && queue.length - 1 - index < LOAD_AHEAD;
}

export type ShortSlot = {
	short: VideoSummary;
	/** Position relative to the current short: -1 above, 1 below. */
	offset: -1 | 0 | 1;
	/** Load the stream ahead. The previous short keeps a player it already has, but never gets one. */
	preload: boolean;
};

/** The shorts stacked in the viewer, so both neighbours can slide in. */
export function shortSlots(
	current: VideoSummary,
	previous: VideoSummary | undefined,
	next: VideoSummary | undefined
): ShortSlot[] {
	const slots: ShortSlot[] = [{ short: current, offset: 0, preload: true }];
	if (previous) slots.unshift({ short: previous, offset: -1, preload: false });
	if (next) slots.push({ short: next, offset: 1, preload: true });
	return slots;
}
