import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { activeRoom, addToRoom, announceRoom } from './active-room.svelte';

const VIDEO = 'dQw4w9WgXcQ';

beforeEach(() => {
	const store = new Map<string, string>();
	vi.stubGlobal('localStorage', {
		getItem: (key: string) => store.get(key) ?? null,
		setItem: (key: string, value: string) => store.set(key, value),
		removeItem: (key: string) => store.delete(key)
	});
	vi.stubGlobal('window', new EventTarget());
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('active room', () => {
	it('is handed videos by other tabs, once however many room tabs are open', async () => {
		const first: string[] = [];
		const second: string[] = [];
		const stopFirst = announceRoom({ id: 'room', owner: 'mara' }, (id) => first.push(id));
		const stopSecond = announceRoom({ id: 'room', owner: 'mara' }, (id) => second.push(id));
		const room = activeRoom();
		expect(room).toMatchObject({ id: 'room', owner: 'mara' });

		await expect(addToRoom(room!, VIDEO)).resolves.toBe(true);
		expect(first).toEqual([]);
		expect(second).toEqual([VIDEO]);

		stopSecond();
		expect(activeRoom()).toBeNull();
		stopFirst();
	});

	it('reports when no room tab takes the video', async () => {
		vi.useFakeTimers();
		const pending = addToRoom({ id: 'gone', owner: 'mara', tab: 'x' }, VIDEO);
		await vi.advanceTimersByTimeAsync(2500);
		await expect(pending).resolves.toBe(false);
		vi.useRealTimers();
	});

	it('forgets a room tab that stopped its heartbeat', () => {
		vi.useFakeTimers();
		const stop = announceRoom({ id: 'room', owner: 'mara' }, () => {});
		expect(activeRoom()).not.toBeNull();
		localStorage.setItem(
			'oxistream:together-active',
			JSON.stringify({ id: 'room', owner: 'mara', tab: 'old', at: Date.now() - 60_000 })
		);
		expect(activeRoom()).toBeNull();
		stop();
		vi.useRealTimers();
	});
});
