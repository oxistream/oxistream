import { describe, expect, it } from 'vitest';
import type { VideoSummary } from '#lib/video.js';
import {
	chunk,
	groupByDay,
	progressRatio,
	toHistoryVideo,
	withProgress,
	type HistoryVideo
} from './history-list';

describe('progressRatio', () => {
	it('is the watched share, clamped, and full once finished', () => {
		expect(progressRatio({ position: 50, duration: 200, finished: false })).toBe(0.25);
		expect(progressRatio({ position: 250, duration: 200, finished: false })).toBe(1);
		expect(progressRatio({ position: 10, duration: 200, finished: true })).toBe(1);
		expect(progressRatio({ position: 10, duration: 0, finished: false })).toBe(0);
	});
});

describe('toHistoryVideo', () => {
	it('maps the API item with progress and watch time', () => {
		const video = toHistoryVideo({
			video: {
				id: 'aaaaaaaaaaa',
				title: 'Forging a hammer',
				duration_seconds: 200,
				thumbnails: [{ url: '/proxy/image/a', width: 480, height: 360 }],
				channel: { id: 'UCa', name: 'Alpha', avatar: [], verified: false },
				is_live: false,
				is_short: false,
				is_upcoming: false
			},
			progress: {
				video_id: 'aaaaaaaaaaa',
				position_seconds: 50,
				duration_seconds: 200,
				finished: false,
				updated_at: '2026-11-26T10:00:00Z'
			}
		});
		expect(video).toMatchObject({
			id: 'aaaaaaaaaaa',
			title: 'Forging a hammer',
			durationSeconds: 200,
			thumbnailUrl: '/proxy/image/a',
			channel: { id: 'UCa', name: 'Alpha' },
			progress: 0.25,
			watchedAt: '2026-11-26T10:00:00Z'
		});
	});
});

describe('withProgress', () => {
	it('adds progress only to started videos', () => {
		const videos = [{ id: 'a' }, { id: 'b' }] as VideoSummary[];
		const result = withProgress(videos, (id) => (id === 'a' ? 0.5 : undefined));
		expect(result[0].progress).toBe(0.5);
		expect(result[1]).toBe(videos[1]);
	});
});

describe('chunk', () => {
	it('splits into stable parts', () => {
		expect(chunk([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
		expect(chunk([], 2)).toEqual([]);
	});
});

describe('groupByDay', () => {
	const entry = (id: string, watchedAt: Date) => ({ id, watchedAt: watchedAt.toISOString() });
	const now = new Date(2026, 10, 26, 18, 0);

	it('labels today and yesterday and formats older days', () => {
		const videos = [
			entry('a', new Date(2026, 10, 26, 17, 0)),
			entry('b', new Date(2026, 10, 26, 0, 5)),
			entry('c', new Date(2026, 10, 25, 23, 0)),
			entry('d', new Date(2026, 10, 20, 12, 0)),
			entry('e', new Date(2025, 0, 2, 12, 0))
		] as HistoryVideo[];
		const days = groupByDay(videos, {
			now,
			locale: 'en-US',
			today: 'Today',
			yesterday: 'Yesterday'
		});
		expect(days.map((d) => d.label)).toEqual([
			'Today',
			'Yesterday',
			'Friday, November 20',
			'January 2, 2025'
		]);
		expect(days[0].videos.map((v) => v.id)).toEqual(['a', 'b']);
	});

	it('uses the locale for older days', () => {
		const videos = [entry('a', new Date(2026, 10, 20, 12, 0))] as HistoryVideo[];
		const [day] = groupByDay(videos, { now, locale: 'de', today: 'Heute', yesterday: 'Gestern' });
		expect(day.label).toBe('Freitag, 20. November');
	});
});
