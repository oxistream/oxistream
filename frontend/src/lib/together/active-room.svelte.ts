import { createSubscriber } from 'svelte/reactivity';
import { readJson, writeJson } from '#lib/components/player/storage.js';
import { isVideoId } from '#lib/youtube.js';

/**
 * A room open in some tab of this browser. Other tabs offer "Add to room" and hand the video to
 * that tab over a BroadcastChannel, which queues it over its own connection, so this works for
 * guests too and only while the room is actually open.
 */
export type ActiveRoom = {
	id: string;
	owner: string;
	/** The tab that announced it, so only that one queues what other tabs send. */
	tab: string;
};

const KEY = 'oxistream:together-active';
const CHANNEL = 'oxistream:together';
const HEARTBEAT_MS = 10_000;
/** A room tab that missed this many heartbeats is gone, e.g. closed without cleanup. */
const STALE_MS = 3 * HEARTBEAT_MS;
const ACK_TIMEOUT_MS = 2000;

type Saved = ActiveRoom & { at: number };
type ChannelMessage =
	| { type: 'add'; room: string; tab: string; videoId: string; request: string }
	| { type: 'added'; request: string };

const isSaved = (v: unknown): v is Saved =>
	typeof v === 'object' &&
	v !== null &&
	typeof (v as Saved).id === 'string' &&
	typeof (v as Saved).owner === 'string' &&
	typeof (v as Saved).tab === 'string' &&
	typeof (v as Saved).at === 'number';

function read(now = Date.now()): ActiveRoom | null {
	const saved = readJson(KEY, isSaved);
	if (!saved || now - saved.at > STALE_MS || saved.at > now + STALE_MS) return null;
	return { id: saved.id, owner: saved.owner, tab: saved.tab };
}

function openChannel(): BroadcastChannel | null {
	return typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel(CHANNEL);
}

const subscribe = createSubscriber((update) => {
	const onStorage = (event: StorageEvent) => {
		if (event.key === KEY || event.key === null) update();
	};
	window.addEventListener('storage', onStorage);
	const timer = setInterval(update, HEARTBEAT_MS);
	return () => {
		window.removeEventListener('storage', onStorage);
		clearInterval(timer);
	};
});

/** The room open in another tab, if any; reactive. */
export function activeRoom(): ActiveRoom | null {
	subscribe();
	return read();
}

/**
 * Marks `room` as open while this tab is connected to it and queues videos other tabs send.
 * Returns a function that withdraws it.
 */
export function announceRoom(
	{ id, owner }: Omit<ActiveRoom, 'tab'>,
	onadd: (videoId: string) => void
): () => void {
	const room: ActiveRoom = { id, owner, tab: Math.random().toString(36).slice(2) };
	const beat = () => writeJson(KEY, { ...room, at: Date.now() } satisfies Saved);
	beat();
	const timer = setInterval(beat, HEARTBEAT_MS);
	const channel = openChannel();
	if (channel) {
		channel.onmessage = (event: MessageEvent<ChannelMessage>) => {
			const message = event.data;
			if (message?.type !== 'add' || message.tab !== room.tab || message.room !== room.id) return;
			if (!isVideoId(message.videoId)) return;
			onadd(message.videoId);
			channel.postMessage({ type: 'added', request: message.request } satisfies ChannelMessage);
		};
	}
	const withdraw = () => {
		if (readJson(KEY, isSaved)?.tab !== room.tab) return;
		try {
			localStorage.removeItem(KEY);
		} catch {
			// Storage may be unavailable; the entry then goes stale.
		}
	};
	// Closing the tab does not tear down components.
	window.addEventListener('pagehide', withdraw);
	return () => {
		clearInterval(timer);
		channel?.close();
		window.removeEventListener('pagehide', withdraw);
		withdraw();
	};
}

/** Hands `videoId` to the tab with `room` open; resolves whether that tab took it. */
export function addToRoom(room: ActiveRoom, videoId: string): Promise<boolean> {
	const channel = openChannel();
	if (!channel) return Promise.resolve(false);
	const request = Math.random().toString(36).slice(2);
	return new Promise((resolve) => {
		const done = (ok: boolean) => {
			clearTimeout(timer);
			channel.close();
			resolve(ok);
		};
		const timer = setTimeout(() => done(false), ACK_TIMEOUT_MS);
		channel.onmessage = (event: MessageEvent<ChannelMessage>) => {
			if (event.data?.type === 'added' && event.data.request === request) done(true);
		};
		channel.postMessage({
			type: 'add',
			room: room.id,
			tab: room.tab,
			videoId,
			request
		} satisfies ChannelMessage);
	});
}
