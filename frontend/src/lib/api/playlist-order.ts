/** Local playlist IDs: 16 URL-safe characters. YouTube's (`PL…`, `RD…`, …) never match. */
export function isLocalPlaylistId(id: string | null | undefined): id is string {
	return typeof id === 'string' && /^[A-Za-z0-9_-]{16}$/.test(id);
}

/** The liked videos list, under YouTube's ID for it so `/playlist?list=LL` links keep working. */
export const LIKED_LIST_ID = 'LL';

export function isLikedListId(id: string | null | undefined): boolean {
	return id === LIKED_LIST_ID;
}

/** A copy of `items` with the item at `from` moved to `to`, both clamped to the list. */
export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
	const next = [...items];
	if (from < 0 || from >= next.length) return next;
	const target = Math.max(0, Math.min(to, next.length - 1));
	const [item] = next.splice(from, 1);
	next.splice(target, 0, item);
	return next;
}

/**
 * Where a dragged item lands: the number of other items whose vertical midpoint lies above
 * `y`. `midpoints` are measured in the original order, before the drag started.
 */
export function dropIndex(midpoints: readonly number[], dragged: number, y: number): number {
	let index = 0;
	midpoints.forEach((mid, i) => {
		if (i !== dragged && mid < y) index++;
	});
	return index;
}

export type Neighbors<T> = { index: number; previous?: T; next?: T };

/** The position of `currentId` in the list and the items around it; `index` is -1 if absent. */
export function neighbors<T extends { id: string }>(
	items: readonly T[],
	currentId: string | null | undefined
): Neighbors<T> {
	const index = currentId ? items.findIndex((item) => item.id === currentId) : -1;
	if (index < 0) return { index };
	return { index, previous: items[index - 1], next: items[index + 1] };
}

/** The video to continue from: the last played one if still listed, else the first. */
export function resumeVideo<T extends { id: string }>(
	items: readonly T[],
	lastVideoId: string | null | undefined
): T | undefined {
	return items.find((item) => item.id === lastVideoId) ?? items[0];
}

/** YouTube mixes (`RD…`) are generated per viewer; only curated `RDCLAK…` lists have a page. */
export function isMixId(id: string | null | undefined): boolean {
	return typeof id === 'string' && id.startsWith('RD') && !id.startsWith('RDCLAK');
}

/**
 * Whether more of a paginated playlist must be loaded for playback: the current video is not
 * among the loaded items yet, or is so close to the end that the next one may be missing.
 */
export function needsMoreItems(
	items: readonly { id: string }[],
	currentId: string | null | undefined,
	lookahead = 2
): boolean {
	if (!currentId) return false;
	const index = items.findIndex((item) => item.id === currentId);
	return index < 0 || index >= items.length - lookahead;
}
