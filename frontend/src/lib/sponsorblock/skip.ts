import {
	isSkipCategory,
	type Segment,
	type SegmentCategory,
	type SponsorBlockSettings
} from './settings';

/** A segment as the player handles it, with the user's preference already applied. */
export type PlayerSegment = {
	id: string;
	category: SegmentCategory;
	/** `skip` ranges are skipped automatically or offer a button; `highlight` is a point. */
	kind: 'skip' | 'highlight' | 'full';
	mode?: 'auto' | 'button';
	start: number;
	end: number;
};

/** Skipping the last bit of a segment is not worth the jump. */
const MIN_REMAINING = 0.5;

export function resolveSegments(
	segments: Segment[],
	settings: SponsorBlockSettings
): PlayerSegment[] {
	const resolved: PlayerSegment[] = [];
	for (const s of segments) {
		const base = { id: s.id, category: s.category, start: s.start, end: s.end };
		if (s.action === 'skip' && isSkipCategory(s.category)) {
			const mode = settings.categories[s.category];
			if (mode !== 'off') resolved.push({ ...base, kind: 'skip', mode });
		} else if (s.action === 'poi' && s.category === 'poi_highlight' && settings.highlight) {
			resolved.push({ ...base, kind: 'highlight' });
		} else if (s.action === 'full' && settings.full_video_label) {
			resolved.push({ ...base, kind: 'full' });
		}
	}
	return resolved;
}

export type AutoSkip = { segment: PlayerSegment; target: number };

/**
 * Decides when to skip. A segment the user seeks into, or whose skip they undo, is left alone
 * until the next video.
 */
export function createSkipper(segments: PlayerSegment[]) {
	const skippable = segments.filter((s) => s.kind === 'skip');
	const auto = skippable.filter((s) => s.mode === 'auto');
	const suppressed = new Set<string>();
	const inside = (s: PlayerSegment, time: number) =>
		time >= s.start && time < s.end - MIN_REMAINING;

	function autoSkip(time: number): AutoSkip | null {
		const segment = auto.find((s) => !suppressed.has(s.id) && inside(s, time));
		if (!segment) return null;
		let target = segment.end;
		for (let extended = true; extended;) {
			extended = false;
			for (const s of auto) {
				if (!suppressed.has(s.id) && s.start <= target && s.end > target) {
					target = s.end;
					extended = true;
				}
			}
		}
		return { segment, target };
	}

	return {
		/** The automatic skip due at `time`, through any overlapping automatic segments. */
		autoSkip,
		/** Where the next automatic skip lands, if one starts within `lead` seconds after `time`. */
		upcoming(time: number, lead: number): number | null {
			let next: PlayerSegment | undefined;
			for (const s of auto) {
				if (suppressed.has(s.id) || s.start <= time || s.start - time > lead) continue;
				if (!next || s.start < next.start) next = s;
			}
			return next ? (autoSkip(next.start)?.target ?? null) : null;
		},
		/** The segment offering a skip button at `time`. */
		button(time: number): PlayerSegment | undefined {
			return skippable.find(
				(s) => inside(s, time) && (s.mode === 'button' || suppressed.has(s.id))
			);
		},
		/** The user moved the playhead to `time` themselves. */
		seeked(time: number) {
			for (const s of auto) if (inside(s, time)) suppressed.add(s.id);
		},
		suppress(id: string) {
			suppressed.add(id);
		}
	};
}

export type Skipper = ReturnType<typeof createSkipper>;

/**
 * The segment under the pointer at `time`: the shortest range containing it, else a highlight
 * within `tolerance` seconds, since a point is hard to hit exactly.
 */
export function segmentAt(
	segments: PlayerSegment[],
	time: number,
	tolerance = 0
): PlayerSegment | undefined {
	let best: PlayerSegment | undefined;
	for (const s of segments) {
		if (s.kind !== 'skip' || time < s.start || time >= s.end) continue;
		if (!best || s.end - s.start < best.end - best.start) best = s;
	}
	if (best) return best;
	let nearest: PlayerSegment | undefined;
	for (const s of segments) {
		if (s.kind !== 'highlight') continue;
		const distance = Math.abs(time - s.start);
		if (distance <= tolerance && (!nearest || distance < Math.abs(time - nearest.start))) {
			nearest = s;
		}
	}
	return nearest;
}
