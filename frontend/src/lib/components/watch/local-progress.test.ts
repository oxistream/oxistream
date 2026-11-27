import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	PROGRESS_KEY,
	readLocalProgress,
	recentLocalVideos,
	removeLocalProgress,
	resumePosition,
	saveLocalProgress
} from './local-progress';

function memoryStorage(): Storage {
	const data = new Map<string, string>();
	return {
		get length() {
			return data.size;
		},
		clear: () => data.clear(),
		getItem: (key) => data.get(key) ?? null,
		key: (i) => [...data.keys()][i] ?? null,
		removeItem: (key) => void data.delete(key),
		setItem: (key, value) => void data.set(key, String(value))
	};
}

describe('local progress', () => {
	beforeEach(() => {
		vi.stubGlobal('localStorage', memoryStorage());
	});

	it('stores and reads progress per video', () => {
		saveLocalProgress('a', { position: 30, duration: 100, finished: false, reason: 'pause' }, 1);
		expect(readLocalProgress('a')).toEqual({
			position: 30,
			duration: 100,
			finished: false,
			updatedAt: 1
		});
		expect(readLocalProgress('b')).toBeNull();
	});

	it('survives broken or blocked storage', () => {
		localStorage.setItem(PROGRESS_KEY, '{not json');
		expect(readLocalProgress('a')).toBeNull();
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new Error('blocked');
			},
			setItem: () => {
				throw new Error('blocked');
			}
		});
		expect(() =>
			saveLocalProgress('a', { position: 1, duration: 2, finished: false, reason: 'pause' })
		).not.toThrow();
		expect(readLocalProgress('a')).toBeNull();
	});

	it('lists watched videos with their details, most recent first', () => {
		const video = { title: 'First', channel: { id: 'UC1', name: 'One' }, durationSeconds: 100 };
		const report = { position: 50, duration: 100, finished: false, reason: 'pause' } as const;
		saveLocalProgress('a', report, 1, video);
		saveLocalProgress('b', report, 3, { ...video, title: 'Second' });
		saveLocalProgress('c', report, 2);
		// A later report without details keeps the stored ones.
		saveLocalProgress('a', { ...report, finished: true }, 4);

		const recent = recentLocalVideos(5);
		expect(recent.map((v) => v.id)).toEqual(['a', 'b']);
		expect(recent[0]).toMatchObject({ title: 'First', progress: 1, durationSeconds: 100 });
		expect(recent[1]).toMatchObject({ title: 'Second', progress: 0.5 });
		expect(recentLocalVideos(1)).toHaveLength(1);
	});

	it('removes one video', () => {
		const video = { title: 'First', channel: { id: 'UC1', name: 'One' } };
		const report = { position: 50, duration: 100, finished: false, reason: 'pause' } as const;
		saveLocalProgress('a', report, 1, video);
		saveLocalProgress('b', report, 2, video);
		removeLocalProgress('a');
		removeLocalProgress('missing');
		expect(readLocalProgress('a')).toBeNull();
		expect(recentLocalVideos(5).map((v) => v.id)).toEqual(['b']);
	});

	it('skips malformed video details', () => {
		localStorage.setItem(
			PROGRESS_KEY,
			JSON.stringify({
				a: { position: 1, duration: 2, finished: false, updatedAt: 1, video: { title: 3 } }
			})
		);
		expect(recentLocalVideos(5)).toEqual([]);
	});

	it('resumes unfinished videos a little earlier', () => {
		const entry = { position: 100, duration: 600, finished: false, updatedAt: 0 };
		expect(resumePosition(entry)).toBe(97);
		expect(resumePosition({ ...entry, finished: true })).toBeNull();
		expect(resumePosition({ ...entry, position: 4 })).toBeNull();
		expect(resumePosition(null)).toBeNull();
	});
});
