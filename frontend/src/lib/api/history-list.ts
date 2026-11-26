import type { VideoSummary } from '#lib/video.js';
import { toVideo } from './mapping';
import type { components } from './schema';

type Schemas = components['schemas'];
export type ApiProgress = Schemas['VideoProgress'];
export type ApiHistoryItem = Schemas['HistoryItem'];

export type WatchProgress = {
	position: number;
	duration: number;
	finished: boolean;
	updatedAt: string;
};

export type HistoryVideo = VideoSummary & { watchedAt: string };

export function toProgress(progress: ApiProgress): WatchProgress {
	return {
		position: progress.position_seconds,
		duration: progress.duration_seconds,
		finished: progress.finished,
		updatedAt: progress.updated_at
	};
}

/** Watched share between 0 and 1; finished videos count as fully watched. */
export function progressRatio(progress: Pick<WatchProgress, 'position' | 'duration' | 'finished'>) {
	if (progress.finished) return 1;
	if (!(progress.duration > 0)) return 0;
	return Math.min(1, Math.max(0, progress.position / progress.duration));
}

export function toHistoryVideo(item: ApiHistoryItem): HistoryVideo {
	return {
		...toVideo(item.video),
		progress: progressRatio(toProgress(item.progress)),
		watchedAt: item.progress.updated_at
	};
}

/** Adds the watched share to videos the user has started. */
export function withProgress<T extends VideoSummary>(
	videos: T[],
	lookup: (id: string) => number | undefined
): T[] {
	return videos.map((video) => {
		const progress = lookup(video.id);
		return progress === undefined ? video : { ...video, progress };
	});
}

/** Splits IDs into stable chunks, so appending a page only fetches the new ones. */
export function chunk<T>(items: T[], size: number): T[][] {
	const chunks: T[][] = [];
	for (let i = 0; i < items.length; i += size) chunks.push(items.slice(i, i + size));
	return chunks;
}

export type HistoryDay = { key: string; label: string; videos: HistoryVideo[] };

function dayKey(date: Date): string {
	return `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`;
}

/**
 * Groups history entries by the local day they were watched, newest first. Entries must already
 * be sorted newest first.
 */
export function groupByDay(
	videos: HistoryVideo[],
	{
		now = new Date(),
		locale,
		today,
		yesterday
	}: { now?: Date; locale: string; today: string; yesterday: string }
): HistoryDay[] {
	const todayKey = dayKey(now);
	const before = new Date(now);
	before.setDate(before.getDate() - 1);
	const yesterdayKey = dayKey(before);
	const sameYear = new Intl.DateTimeFormat(locale, {
		weekday: 'long',
		day: 'numeric',
		month: 'long'
	});
	const otherYear = new Intl.DateTimeFormat(locale, { dateStyle: 'long' });

	const days: HistoryDay[] = [];
	for (const video of videos) {
		const date = new Date(video.watchedAt);
		const key = dayKey(date);
		let day = days.at(-1);
		if (day?.key !== key) {
			const label =
				key === todayKey
					? today
					: key === yesterdayKey
						? yesterday
						: (date.getFullYear() === now.getFullYear() ? sameYear : otherYear).format(date);
			day = { key, label, videos: [] };
			days.push(day);
		}
		day.videos.push(video);
	}
	return days;
}
