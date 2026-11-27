import { moveItem } from '#lib/api/playlist-order.js';

const STORAGE_KEY = 'oxistream:queue';
const MAX_ITEMS = 200;

/** A video waiting in the up-next queue. */
export type QueueVideo = {
	id: string;
	title: string;
	channel?: { id: string; name: string } | null;
	thumbnailUrl?: string;
	durationSeconds?: number;
};

export type QueueEntry = QueueVideo & {
	/** Tells entries of the same video apart. */
	key: string;
};

function isEntry(value: unknown): value is QueueEntry {
	if (typeof value !== 'object' || value === null) return false;
	const v = value as Record<string, unknown>;
	return typeof v.key === 'string' && typeof v.id === 'string' && typeof v.title === 'string';
}

function read(storage: Storage | undefined): QueueEntry[] {
	try {
		const raw = storage?.getItem(STORAGE_KEY);
		if (!raw) return [];
		const value: unknown = JSON.parse(raw);
		return Array.isArray(value) ? value.filter(isEntry).slice(0, MAX_ITEMS) : [];
	} catch {
		return [];
	}
}

function sessionStore(): Storage | undefined {
	try {
		return typeof sessionStorage === 'undefined' ? undefined : sessionStorage;
	} catch {
		return undefined;
	}
}

/** The videos the app-wide player plays next, kept per tab. */
export class UpNextQueue {
	entries = $state.raw<QueueEntry[]>([]);
	#storage: Storage | undefined;
	#counter = 0;

	/** `null` keeps the queue in memory only. */
	constructor(storage: Storage | null = sessionStore() ?? null) {
		this.#storage = storage ?? undefined;
		this.entries = read(this.#storage);
	}

	get length() {
		return this.entries.length;
	}

	has(videoId: string) {
		return this.entries.some((e) => e.id === videoId);
	}

	/** Adds to the end. */
	add(video: QueueVideo) {
		this.#set([...this.entries, this.#entry(video)]);
	}

	/** Adds to the front, so it plays after the current video. */
	addNext(video: QueueVideo) {
		this.#set([this.#entry(video), ...this.entries]);
	}

	/** Takes the first entry off the queue. */
	shift(): QueueEntry | undefined {
		const [first, ...rest] = this.entries;
		if (first) this.#set(rest);
		return first;
	}

	/** Takes `key` off the queue, wherever it is. */
	take(key: string): QueueEntry | undefined {
		const entry = this.entries.find((e) => e.key === key);
		if (entry) this.remove(key);
		return entry;
	}

	remove(key: string) {
		this.#set(this.entries.filter((e) => e.key !== key));
	}

	move(from: number, to: number) {
		this.#set(moveItem(this.entries, from, to));
	}

	clear() {
		this.#set([]);
	}

	#entry(video: QueueVideo): QueueEntry {
		const { id, title, channel, thumbnailUrl, durationSeconds } = video;
		const key = `${Date.now().toString(36)}-${(this.#counter++).toString(36)}`;
		return {
			key,
			id,
			title,
			channel: channel ? { id: channel.id, name: channel.name } : null,
			thumbnailUrl,
			durationSeconds
		};
	}

	#set(entries: QueueEntry[]) {
		this.entries = entries.slice(0, MAX_ITEMS);
		try {
			if (this.entries.length) this.#storage?.setItem(STORAGE_KEY, JSON.stringify(this.entries));
			else this.#storage?.removeItem(STORAGE_KEY);
		} catch {
			// Storage unavailable or full; the queue lasts until the tab reloads.
		}
	}
}
