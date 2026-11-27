import { progressRatio } from '#lib/api/history-list.js';
import type { ProgressReport } from '#lib/components/player/progress.js';
import { readJson, writeJson } from '#lib/components/player/storage.js';
import type { VideoSummary } from '#lib/video.js';

export const PROGRESS_KEY = 'oxistream:progress';
const MAX_ENTRIES = 500;

export type StoredProgress = {
	position: number;
	duration: number;
	finished: boolean;
	updatedAt: number;
	/** What the home page needs to list the video again without asking the server. */
	video?: LocalVideo;
};

export type LocalVideo = Pick<
	VideoSummary,
	'title' | 'durationSeconds' | 'isLive' | 'thumbnailUrl'
> & { channel: { id: string; name: string } };

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

function isLocalVideo(value: unknown): value is LocalVideo {
	if (typeof value !== 'object' || value === null) return false;
	const v = value as Record<string, unknown>;
	const channel = v.channel as Record<string, unknown> | null | undefined;
	return (
		typeof v.title === 'string' &&
		typeof channel === 'object' &&
		channel !== null &&
		typeof channel.id === 'string' &&
		typeof channel.name === 'string'
	);
}

/** Watch progress of anonymous users, kept in this browser only. */
export function readLocalProgress(videoId: string): StoredProgress | null {
	const entry = readJson(PROGRESS_KEY, isProgressMap)?.[videoId];
	return isEntry(entry) ? entry : null;
}

export function saveLocalProgress(
	videoId: string,
	report: ProgressReport,
	now = Date.now(),
	video?: LocalVideo
): void {
	const map = readJson(PROGRESS_KEY, isProgressMap) ?? {};
	const previous = map[videoId];
	map[videoId] = {
		position: report.position,
		duration: report.duration,
		finished: report.finished,
		updatedAt: now,
		video: video ?? (isEntry(previous) && isLocalVideo(previous.video) ? previous.video : undefined)
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

export function removeLocalProgress(videoId: string): void {
	const map = readJson(PROGRESS_KEY, isProgressMap);
	if (!map || !(videoId in map)) return;
	delete map[videoId];
	writeJson(PROGRESS_KEY, map);
}

/** Videos watched in this browser that can be listed again, most recent first. */
export function recentLocalVideos(limit: number): VideoSummary[] {
	const map = readJson(PROGRESS_KEY, isProgressMap) ?? {};
	return Object.entries(map)
		.filter((entry): entry is [string, StoredProgress & { video: LocalVideo }] => {
			const value = entry[1];
			return isEntry(value) && isLocalVideo(value.video);
		})
		.sort(([, a], [, b]) => b.updatedAt - a.updatedAt)
		.slice(0, limit)
		.map(([id, { video, ...progress }]) => ({
			id,
			title: video.title,
			channel: { id: video.channel.id, name: video.channel.name },
			durationSeconds:
				typeof video.durationSeconds === 'number' ? video.durationSeconds : undefined,
			isLive: video.isLive === true,
			thumbnailUrl: typeof video.thumbnailUrl === 'string' ? video.thumbnailUrl : undefined,
			progress: progressRatio(progress)
		}));
}

/** Where to pick up again: unfinished videos resume a few seconds back, finished ones from the start. */
export function resumePosition(
	progress: Pick<StoredProgress, 'position' | 'finished'> | null
): number | null {
	if (!progress || progress.finished || progress.position < 10) return null;
	return Math.max(0, Math.floor(progress.position - 3));
}
