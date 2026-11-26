import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	PROGRESS_KEY,
	readLocalProgress,
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

	it('resumes unfinished videos a little earlier', () => {
		const entry = { position: 100, duration: 600, finished: false, updatedAt: 0 };
		expect(resumePosition(entry)).toBe(97);
		expect(resumePosition({ ...entry, finished: true })).toBeNull();
		expect(resumePosition({ ...entry, position: 4 })).toBeNull();
		expect(resumePosition(null)).toBeNull();
	});
});
