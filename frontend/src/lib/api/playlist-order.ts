/** Local playlist IDs: 16 URL-safe characters. YouTube's (`PL…`, `RD…`, …) never match. */
export function isLocalPlaylistId(id: string | null | undefined): id is string {
	return typeof id === 'string' && /^[A-Za-z0-9_-]{16}$/.test(id);
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
