import type { ProgressReport } from '#lib/components/player/progress.js';
import { readJson, writeJson } from '#lib/components/player/storage.js';

export const PROGRESS_KEY = 'oxistream:progress';
const MAX_ENTRIES = 500;

export type StoredProgress = {
	position: number;
	duration: number;
	finished: boolean;
	updatedAt: number;
};

type ProgressMap = Record<string, unknown>;

function isProgressMap(value: unknown): value is ProgressMap {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isEntry(value: unknown): value is StoredProgress {
	if (typeof value !== 'object' || value === null) return false;
	const v = value as Record<string, unknown>;
	return (
		typeof v.position === 'number' &&
		typeof v.duration === 'number' &&
		typeof v.finished === 'boolean' &&
		typeof v.updatedAt === 'number'
	);
}

/** Watch progress of anonymous users, kept in this browser only. */
export function readLocalProgress(videoId: string): StoredProgress | null {
	const entry = readJson(PROGRESS_KEY, isProgressMap)?.[videoId];
	return isEntry(entry) ? entry : null;
}

export function saveLocalProgress(videoId: string, report: ProgressReport, now = Date.now()): void {
	const map = readJson(PROGRESS_KEY, isProgressMap) ?? {};
	map[videoId] = {
		position: report.position,
		duration: report.duration,
		finished: report.finished,
		updatedAt: now
	} satisfies StoredProgress;
	const entries = Object.entries(map).filter((entry): entry is [string, StoredProgress] =>
		isEntry(entry[1])
	);
	if (entries.length > MAX_ENTRIES) {
		entries.sort(([, a], [, b]) => b.updatedAt - a.updatedAt);
		entries.length = MAX_ENTRIES;
	}
	writeJson(PROGRESS_KEY, Object.fromEntries(entries));
}

/** Where to pick up again: unfinished videos resume a few seconds back, finished ones from the start. */
export function resumePosition(progress: StoredProgress | null): number | null {
	if (!progress || progress.finished || progress.position < 10) return null;
	return Math.max(0, Math.floor(progress.position - 3));
}
