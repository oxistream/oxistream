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

/** Ask for more shorts once the viewer gets this close to the end of the queue. */
export const LOAD_AHEAD = 3;

export function needsMore(queue: VideoSummary[], index: number): boolean {
	return index >= 0 && queue.length - 1 - index < LOAD_AHEAD;
}
