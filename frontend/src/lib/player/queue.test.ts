import { describe, expect, it } from 'vitest';
import { moveItem } from '#lib/api/playlist-order.js';
import {
	QUEUE_STORAGE_KEY,
	UpNextQueue,
	type QueueEntry,
	type QueueOp,
	type QueueRemote,
	type QueueState
} from './queue.svelte';

function memoryStorage(): Storage & { data: Map<string, string> } {
	const data = new Map<string, string>();
	return {
		data,
		getItem: (k: string) => data.get(k) ?? null,
		setItem: (k: string, v: string) => void data.set(k, v),
		removeItem: (k: string) => void data.delete(k)
	} as Storage & { data: Map<string, string> };
}

/** The server's rules: changes go by key, so repeating one does nothing. */
class FakeServer {
	state: QueueState = { revision: 0, entries: [] };
	calls: QueueOp[] = [];

	apply(op: QueueOp): QueueState {
		this.calls.push(op);
		const entries = this.state.entries;
		let next = entries;
		switch (op.type) {
			case 'add': {
				const keys = new Set(entries.map((e) => e.key));
				const videos = new Set(entries.map((e) => e.id));
				const added = op.entries.filter(
					(e) => !keys.has(e.key) && !(op.skipQueued && videos.has(e.id))
				);
				next = op.position === 'start' ? [...added, ...entries] : [...entries, ...added];
				break;
			}
			case 'remove':
				next = entries.filter((e) => e.key !== op.key);
				break;
			case 'move': {
				const from = entries.findIndex((e) => e.key === op.key);
				if (from >= 0) next = moveItem(entries, from, op.index);
				break;
			}
			case 'clear':
				next = [];
		}
		if (next !== entries && JSON.stringify(next) !== JSON.stringify(entries)) {
			this.state = { revision: this.state.revision + 1, entries: next };
		}
		return structuredClone(this.state);
	}

	remote(): QueueRemote & { flush(): Promise<void> } {
		const waiting: (() => void)[] = [];
		return {
			load: () => Promise.resolve(structuredClone(this.state)),
			apply: (op) => new Promise((resolve) => waiting.push(() => resolve(this.apply(op)))),
			async flush() {
				await settle();
				while (waiting.length) {
					waiting.shift()!();
					await settle();
				}
			}
		};
	}
}

const ids = (queue: UpNextQueue) => queue.entries.map((e) => e.id);
const settle = () => new Promise((r) => setTimeout(r));

describe('UpNextQueue', () => {
	it('lasts and is shared by every tab of the browser', () => {
		const storage = memoryStorage();
		const tab = new UpNextQueue(storage);
		tab.connect(null);
		tab.add({ id: 'a', title: 'A' });
		tab.add({ id: 'b', title: 'B' });
		tab.add({ id: 'c', title: 'C' });
		tab.move(2, 0);
		tab.remove(tab.entries[1].key);
		const other = new UpNextQueue(storage);
		expect(ids(other)).toEqual(['c', 'b']);

		tab.addNext({ id: 'd', title: 'D' });
		other.reload();
		expect(ids(other)).toEqual(['d', 'c', 'b']);
		tab.clear();
		expect(storage.data.size).toBe(0);
	});

	it('gives each entry to one tab only', () => {
		const storage = memoryStorage();
		const first = new UpNextQueue(storage);
		const second = new UpNextQueue(storage);
		first.add({ id: 'a', title: 'A' });
		first.add({ id: 'b', title: 'B' });
		second.reload();
		const head = second.entries[0];

		// Both players end at once; the first one takes the head, the second the next one.
		expect(first.shift()?.id).toBe('a');
		expect(second.shift()?.id).toBe('b');
		expect(second.take(head.key)).toBeUndefined();
		first.reload();
		expect(ids(first)).toEqual([]);
	});

	it('merges the guest queue into the account on sign-in', async () => {
		const storage = memoryStorage();
		const server = new FakeServer();
		server.state = {
			revision: 3,
			entries: [{ key: 'server-a', id: 'a', title: 'A' }]
		};
		const queue = new UpNextQueue(storage);
		queue.connect(null);
		queue.add({ id: 'a', title: 'A' });
		queue.add({ id: 'b', title: 'B' });
		const remote = server.remote();

		queue.connect('user-1', remote);
		expect(ids(queue)).toEqual(['a', 'b']);
		await remote.flush();
		expect(server.calls).toMatchObject([{ type: 'add', position: 'end', skipQueued: true }]);
		expect(ids(queue)).toEqual(['a', 'b']);
		expect(queue.entries[0].key).toBe('server-a');
		const stored: unknown = JSON.parse(storage.data.get(QUEUE_STORAGE_KEY)!);
		expect(stored).toMatchObject({ owner: 'user-1', revision: 4 });

		// Another tab of the same user does not merge again.
		const other = new UpNextQueue(storage);
		other.connect('user-1', remote);
		await settle();
		expect(server.calls).toHaveLength(1);
		expect(ids(other)).toEqual(['a', 'b']);

		queue.connect(null);
		expect(ids(queue)).toEqual([]);
		expect(storage.data.size).toBe(0);
	});

	it('shows the server state once the last change is answered', async () => {
		const storage = memoryStorage();
		const server = new FakeServer();
		const remote = server.remote();
		const queue = new UpNextQueue(storage);
		queue.connect('user-1', remote);
		await settle();
		queue.add({ id: 'a', title: 'A' });
		queue.add({ id: 'b', title: 'B' });
		queue.move(1, 0);
		expect(ids(queue)).toEqual(['b', 'a']);

		// Another device adds a video meanwhile.
		server.apply({
			type: 'add',
			entries: [{ key: 'elsewhere', id: 'c', title: 'C' }],
			position: 'end'
		});
		await remote.flush();
		expect(ids(queue)).toEqual(['b', 'c', 'a']);
		expect(server.state.revision).toBe(4);
	});

	it('picks up changes from other devices on refresh', async () => {
		const server = new FakeServer();
		const remote = server.remote();
		const queue = new UpNextQueue(memoryStorage());
		queue.connect('user-1', remote);
		await settle();
		server.apply({
			type: 'add',
			entries: [{ key: 'k', id: 'c', title: 'C' }],
			position: 'end'
		});
		await queue.refresh();
		expect(ids(queue)).toEqual(['c']);
	});

	it('keeps a newer state another tab stored over an older answer', async () => {
		const storage = memoryStorage();
		const server = new FakeServer();
		const remote = server.remote();
		const queue = new UpNextQueue(storage);
		queue.connect('user-1', remote);
		await settle();
		queue.add({ id: 'a', title: 'A' });
		const newer: QueueEntry[] = [{ key: 'x', id: 'x', title: 'X' }];
		storage.setItem(
			QUEUE_STORAGE_KEY,
			JSON.stringify({ owner: 'user-1', revision: 9, entries: newer })
		);
		await remote.flush();
		expect(ids(queue)).toEqual(['x']);
	});

	it('ignores what another account stored', () => {
		const storage = memoryStorage();
		const queue = new UpNextQueue(storage);
		queue.connect('user-1', null);
		storage.setItem(
			QUEUE_STORAGE_KEY,
			JSON.stringify({ owner: 'user-2', revision: 1, entries: [{ key: 'k', id: 'a', title: 'A' }] })
		);
		queue.reload();
		expect(ids(queue)).toEqual([]);
	});

	it('sends changes made before the session was known once it is', async () => {
		const storage = memoryStorage();
		storage.setItem(
			QUEUE_STORAGE_KEY,
			JSON.stringify({ owner: 'user-1', revision: 0, entries: [] })
		);
		const server = new FakeServer();
		const remote = server.remote();
		const queue = new UpNextQueue(storage);
		queue.add({ id: 'a', title: 'A' });
		queue.connect('user-1', remote);
		await remote.flush();
		expect(server.state.entries.map((e) => e.id)).toEqual(['a']);
		expect(ids(queue)).toEqual(['a']);
	});
});
