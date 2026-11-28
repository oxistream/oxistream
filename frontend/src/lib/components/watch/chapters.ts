import type { Thumbnail } from '#lib/api/mapping.js';

export type Chapter = {
	title: string;
	start: number;
	/** Start of the next chapter, or the end of the video. */
	end: number;
	thumbnails: Thumbnail[];
};

type ApiChapter = { title: string; start_seconds: number; thumbnails: Thumbnail[] };

/** Sorted chapters with their ends; chapters past the end of the video are dropped. */
export function toChapters(
	chapters: ApiChapter[] | undefined,
	duration?: number | null
): Chapter[] {
	const end = duration && duration > 0 ? duration : Infinity;
	const sorted = [...(chapters ?? [])]
		.filter((c) => c.start_seconds < end)
		.sort((a, b) => a.start_seconds - b.start_seconds);
	return sorted.map((c, i) => ({
		title: c.title,
		start: c.start_seconds,
		end: sorted[i + 1]?.start_seconds ?? end,
		thumbnails: c.thumbnails
	}));
}

/**
 * Index of the last item starting at or before `time`, or -1 before the first. `items` must be
 * sorted by `start`.
 */
export function indexAt(items: readonly { start: number }[], time: number): number {
	let low = 0;
	let high = items.length - 1;
	let found = -1;
	while (low <= high) {
		const mid = (low + high) >> 1;
		if (items[mid].start <= time) {
			found = mid;
			low = mid + 1;
		} else {
			high = mid - 1;
		}
	}
	return found;
}
