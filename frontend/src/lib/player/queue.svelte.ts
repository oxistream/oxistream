import { moveItem } from '#lib/api/playlist-order.js';

export const QUEUE_STORAGE_KEY = 'oxistream:queue';
export const MAX_QUEUE_ITEMS = 200;

/** A video waiting in the up-next queue. */
export type QueueVideo = {
	id: string;
	title: string;
	channel?: { id: string; name: string } | null;
	thumbnailUrl?: string;
	durationSeconds?: number;
};

export type QueueEntry = QueueVideo & {
	/** Tells entries apart, also across tabs and devices. */
	key: string;
};

export type QueueOp =
	| { type: 'add'; entries: QueueEntry[]; position: 'start' | 'end'; skipQueued?: boolean }
	| { type: 'remove'; key: string }
	| { type: 'move'; key: string; index: number }
	| { type: 'clear' };

/** The server's queue; `revision` grows with every change. */
export type QueueState = { revision: number; entries: QueueEntry[] };

/** Where a signed-in user's queue lives. */
export interface QueueRemote {
	load(): Promise<QueueState>;
	/** Returns the queue after the change. */
	apply(op: QueueOp): Promise<QueueState>;
}

/** What every tab of the browser shares: a guest's queue, or a cached copy of the user's. */
type Stored = { owner: string | null; revision: number; entries: QueueEntry[] };

function isEntry(value: unknown): value is QueueEntry {
	if (typeof value !== 'object' || value === null) return false;
	const v = value as Record<string, unknown>;
	return typeof v.key === 'string' && typeof v.id === 'string' && typeof v.title === 'string';
}

const EMPTY: Stored = { owner: null, revision: 0, entries: [] };

function read(storage: Storage | undefined): Stored {
	try {
		const raw = storage?.getItem(QUEUE_STORAGE_KEY);
		if (!raw) return EMPTY;
		const value = JSON.parse(raw) as Partial<Stored> | null;
		if (!value || !Array.isArray(value.entries)) return EMPTY;
		return {
			owner: typeof value.owner === 'string' ? value.owner : null,
			revision: typeof value.revision === 'number' ? value.revision : 0,
			entries: value.entries.filter(isEntry).slice(0, MAX_QUEUE_ITEMS)
		};
	} catch {
		return EMPTY;
	}
}

function localStore(): Storage | undefined {
	try {
		return typeof localStorage === 'undefined' ? undefined : localStorage;
	} catch {
		return undefined;
	}
}

function randomKey(): string {
	const bytes = new Uint8Array(9);
	globalThis.crypto.getRandomValues(bytes);
	return [...bytes].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * The videos the app-wide player plays next. Every tab shares it through local storage; a
 * signed-in user's queue also lives on the server (see `connect`). Changes refer to entries
 * by key, so when two tabs take the same entry, the second one finds it gone.
 */
export class UpNextQueue {
	entries = $state.raw<QueueEntry[]>([]);
	#storage: Storage | undefined;
	/** Whose queue `entries` is: `null` for a guest, `undefined` until `connect`. */
	#owner: string | null | undefined;
	#remote: QueueRemote | null = null;
	#revision = 0;
	/** Server requests in flight; while any are, their responses are not shown. */
	#pending = 0;
	#chain: Promise<unknown> = Promise.resolve();
	/** Changes made before the session was known, sent once it is. */
	#early: QueueOp[] = [];
	/** Bumped by `connect`, so responses for an earlier user are dropped. */
	#epoch = 0;

	/** `null` keeps the queue in memory only. */
	constructor(storage: Storage | null = localStore() ?? null) {
		this.#storage = storage ?? undefined;
		const stored = read(this.#storage);
		this.entries = stored.entries;
		this.#revision = stored.revision;
		if (this.#storage && typeof window !== 'undefined') {
			window.addEventListener('storage', (event) => {
				if (event.key === QUEUE_STORAGE_KEY || event.key === null) this.reload();
			});
		}
	}

	get length() {
		return this.entries.length;
	}

	has(videoId: string) {
		return this.entries.some((e) => e.id === videoId);
	}

	/**
	 * Says whose queue to show: `owner` is the signed-in user's ID, `null` for guests. Signing
	 * in merges the guest queue into the user's; signing out forgets the user's copy.
	 */
	connect(owner: string | null, remote: QueueRemote | null = null) {
		if (this.#owner === owner && this.#remote === remote) return;
		const stored = read(this.#storage);
		const early = this.#early;
		this.#early = [];
		this.#owner = owner;
		this.#remote = owner === null ? null : remote;
		this.#pending = 0;
		this.#chain = Promise.resolve();
		this.#epoch++;
		if (owner === null) {
			if (stored.owner !== null) this.#write({ owner: null, revision: 0, entries: [] });
			else this.#show(stored);
			return;
		}
		if (stored.owner === owner) {
			this.#show(stored);
			if (early.length) early.forEach((op) => this.#send(op));
			else void this.refresh();
			return;
		}
		// The guest queue joins the end of the user's; videos the user queued already stay put.
		const guest = stored.owner === null ? stored.entries : [];
		this.#revision = 0;
		this.#write({ owner, revision: 0, entries: guest });
		if (guest.length)
			this.#send({ type: 'add', entries: guest, position: 'end', skipQueued: true });
		else void this.refresh();
	}

	/** Fetches the user's queue, e.g. when the tab comes back into view. */
	refresh(): Promise<void> {
		const remote = this.#remote;
		if (!remote || this.#pending > 0) return Promise.resolve();
		return this.#request(() => remote.load(), false);
	}

	/** Takes over what another tab stored. */
	reload() {
		if (!this.#storage) return;
		const stored = read(this.#storage);
		if (this.#owner === undefined || stored.owner === this.#owner) this.#show(stored);
	}

	/** Adds to the end. */
	add(video: QueueVideo) {
		const entry = this.#entry(video);
		this.#change([...this.entries, entry], {
			type: 'add',
			entries: [entry],
			position: 'end'
		});
	}

	/** Adds to the front, so it plays after the current video. */
	addNext(video: QueueVideo) {
		const entry = this.#entry(video);
		this.#change([entry, ...this.entries], {
			type: 'add',
			entries: [entry],
			position: 'start'
		});
	}

	/** Takes the first entry off the queue, unless another tab took it first. */
	shift(): QueueEntry | undefined {
		this.reload();
		const first = this.entries[0];
		if (first) this.remove(first.key);
		return first;
	}

	/** Takes `key` off the queue, wherever it is. */
	take(key: string): QueueEntry | undefined {
		this.reload();
		const entry = this.entries.find((e) => e.key === key);
		if (entry) this.remove(key);
		return entry;
	}

	remove(key: string) {
		if (!this.entries.some((e) => e.key === key)) return;
		this.#change(
			this.entries.filter((e) => e.key !== key),
			{ type: 'remove', key }
		);
	}

	move(from: number, to: number) {
		const entry = this.entries[from];
		if (!entry) return;
		const index = Math.max(0, Math.min(to, this.entries.length - 1));
		if (index === from) return;
		this.#change(moveItem(this.entries, from, index), { type: 'move', key: entry.key, index });
	}

	clear() {
		if (!this.entries.length) return;
		this.#change([], { type: 'clear' });
	}

	#entry(video: QueueVideo): QueueEntry {
		const { id, title, channel, thumbnailUrl, durationSeconds } = video;
		return {
			key: randomKey(),
			id,
			title,
			channel: channel ? { id: channel.id, name: channel.name } : null,
			thumbnailUrl,
			durationSeconds
		};
	}

	#change(entries: QueueEntry[], op: QueueOp) {
		const owner = this.#owner === undefined ? read(this.#storage).owner : this.#owner;
		this.#write({ owner, revision: this.#revision, entries: entries.slice(0, MAX_QUEUE_ITEMS) });
		if (this.#owner === undefined && owner !== null) this.#early.push(op);
		else this.#send(op);
	}

	#send(op: QueueOp) {
		const remote = this.#remote;
		if (remote) void this.#request(() => remote.apply(op), true);
	}

	/**
	 * Runs server requests one after another and shows the state after the last one. A failed
	 * change shows the server's state instead, with `refetch`.
	 */
	#request(run: () => Promise<QueueState>, refetch: boolean): Promise<void> {
		const owner = this.#owner ?? null;
		const epoch = this.#epoch;
		this.#pending++;
		const done = this.#chain.then(run).then(
			(state) => {
				if (this.#epoch !== epoch) return;
				if (--this.#pending === 0) this.#accept(owner, state);
			},
			() => {
				if (this.#epoch !== epoch) return;
				if (--this.#pending === 0 && refetch) void this.refresh();
			}
		);
		this.#chain = done;
		return done;
	}

	#accept(owner: string | null, state: QueueState) {
		const stored = read(this.#storage);
		const newest = stored.owner === owner ? Math.max(stored.revision, this.#revision) : 0;
		// Another tab already stored a newer state.
		if (state.revision < newest) return this.reload();
		this.#write({ owner, revision: state.revision, entries: state.entries });
	}

	#show(stored: Stored) {
		this.entries = stored.entries;
		this.#revision = stored.revision;
	}

	#write(stored: Stored) {
		this.#show(stored);
		try {
			if (stored.entries.length || stored.owner !== null) {
				this.#storage?.setItem(QUEUE_STORAGE_KEY, JSON.stringify(stored));
			} else {
				this.#storage?.removeItem(QUEUE_STORAGE_KEY);
			}
		} catch {
			// Storage unavailable or full; the queue lasts until the tab reloads.
		}
	}
}
