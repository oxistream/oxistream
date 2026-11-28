import type { components } from '../api/schema';
import { ClockSync, backoffDelay, localNow } from './sync';

type Schemas = components['schemas'];
export type RoomState = Schemas['RoomState'];
export type RoomQueueEntry = Schemas['QueueEntry'];
export type RoomParticipant = Schemas['RoomParticipant'];
export type RoomErrorCode = Schemas['RoomErrorCode'];
export type RoomClientMessage = Schemas['RoomClientMessage'];
export type MemberSync = Schemas['MemberSync'];
export type MemberPlace = Schemas['MemberPlace'];
export type MemberStatus = { sync: MemberSync | null; place: MemberPlace };
type ServerMessage = Schemas['RoomServerMessage'];
type KnownStatus = { sync: MemberSync | null; place: MemberPlace | null; revision: number };

export type ConnectionStatus =
	| 'connecting'
	| 'open'
	| 'reconnecting'
	/** Final states: the connection is not retried. */
	| 'closed'
	| 'kicked'
	| 'not_found'
	| 'full'
	| 'rejected';

export type JoinDetails = { name?: string; guestKey?: string };

/** The most videos one `add_many` may carry, the queue limit. */
export const MAX_BATCH = 100;
const PING_INTERVAL = 10_000;
/** Pings right after joining, so the clock is accurate before the first seek. */
const PING_BURST = [0, 150, 300, 450, 600, 900];
const FINAL_ERRORS: Partial<Record<RoomErrorCode, ConnectionStatus>> = {
	not_found: 'not_found',
	room_full: 'full'
};

export function socketUrl(roomId: string, location: Location = window.location): string {
	const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${scheme}//${location.host}/api/v0/rooms/${encodeURIComponent(roomId)}/ws`;
}

/** A room's WebSocket, reconnecting with backoff until the room is gone or the user left. */
export class RoomConnection {
	status = $state<ConnectionStatus>('connecting');
	room = $state<RoomState | null>(null);
	/** Played entries, newest first. */
	history = $state.raw<RoomQueueEntry[]>([]);
	participantId = $state<string | null>(null);
	isOwner = $state(false);
	/** The last error the server reported for something this client sent. */
	lastError = $state<{ code: RoomErrorCode; at: number } | null>(null);
	/** The answer to the last batch this client queued. */
	lastAdded = $state<{ added: number; skipped: number; at: number } | null>(null);

	#roomId: string;
	#join: JoinDetails;
	#socket: WebSocket | undefined;
	#attempt = 0;
	#retry: ReturnType<typeof setTimeout> | undefined;
	#ping: ReturnType<typeof setInterval> | undefined;
	#clock = new ClockSync();
	#welcomeOffset = 0;
	#stopped = false;
	/** Videos cut off from the last batch sent, counted as skipped when its answer arrives. */
	#overflow = 0;
	/** What this client reported last, sent again after reconnecting. */
	#status: MemberStatus | null = null;
	/** The newest status heard of per participant, which may arrive before their state. */
	#statuses: Partial<Record<string, KnownStatus>> = {};

	constructor(roomId: string, join: JoinDetails) {
		this.#roomId = roomId;
		this.#join = join;
	}

	get final(): boolean {
		return !['connecting', 'open', 'reconnecting'].includes(this.status);
	}

	/** The server's clock in ms since the epoch. */
	serverNow(): number {
		return localNow() + this.#clock.offset(this.#welcomeOffset);
	}

	/** How well the server clock is known, for the sync readout. */
	clockStats(): { offset: number; rtt: number | null; samples: number } {
		const clock = this.#clock;
		return { offset: clock.offset(this.#welcomeOffset), rtt: clock.rtt, samples: clock.samples };
	}

	connect() {
		this.#stopped = false;
		const socket = new WebSocket(socketUrl(this.#roomId));
		this.#socket = socket;
		socket.addEventListener('open', () => {
			this.#write({ type: 'join', name: this.#join.name, guest_key: this.#join.guestKey });
		});
		socket.addEventListener('message', (event) => {
			if (typeof event.data === 'string') this.#receive(event.data);
		});
		socket.addEventListener('close', () => {
			if (this.#socket !== socket) return;
			this.#socket = undefined;
			clearInterval(this.#ping);
			if (this.#stopped || this.final) return;
			this.status = 'reconnecting';
			this.#retry = setTimeout(() => this.connect(), backoffDelay(this.#attempt++));
		});
	}

	/** Leaves the room for good. */
	disconnect() {
		this.#stopped = true;
		clearTimeout(this.#retry);
		clearInterval(this.#ping);
		this.#socket?.close(1000);
		this.#socket = undefined;
	}

	send(message: Exclude<RoomClientMessage, { type: 'join' | 'ping' }>) {
		this.#write(message);
	}

	/** Queues up to {@link MAX_BATCH} videos at once; the rest count as skipped. */
	addMany(videoIds: string[]) {
		if (videoIds.length === 0) return;
		if (!this.#write({ type: 'add_many', video_ids: videoIds.slice(0, MAX_BATCH) })) return;
		this.#overflow = Math.max(0, videoIds.length - MAX_BATCH);
	}

	/** Tells the room how this player keeps up and where this tab is. */
	reportStatus(status: MemberStatus) {
		this.#status = status;
		this.#write({ type: 'status', ...status });
	}

	#applyStatus(id: string, status: KnownStatus) {
		const known = this.#statuses[id];
		if (known && known.revision >= status.revision) return;
		this.#statuses[id] = status;
		const person = this.room?.participants.find((p) => p.id === id);
		if (person && person.status_revision < status.revision) {
			person.sync = status.sync;
			person.place = status.place;
			person.status_revision = status.revision;
		}
	}

	/** A state may be older than statuses already heard of. */
	#withStatuses(room: RoomState): RoomState {
		for (const person of room.participants) {
			const known = this.#statuses[person.id];
			if (known && known.revision > person.status_revision) {
				person.sync = known.sync;
				person.place = known.place;
				person.status_revision = known.revision;
			}
		}
		return room;
	}

	#write(message: RoomClientMessage): boolean {
		if (this.#socket?.readyState !== WebSocket.OPEN) return false;
		this.#socket.send(JSON.stringify(message));
		return true;
	}

	#sendPing() {
		this.#write({ type: 'ping', t: localNow() });
	}

	#receive(data: string) {
		let message: ServerMessage;
		try {
			message = JSON.parse(data) as ServerMessage;
		} catch {
			return;
		}
		switch (message.type) {
			case 'welcome':
				this.#attempt = 0;
				// Another replica may answer after a reconnect.
				this.#clock.reset();
				this.#welcomeOffset = message.server_time - localNow();
				this.participantId = message.participant_id;
				this.isOwner = message.is_owner;
				this.status = 'open';
				for (const delay of PING_BURST) setTimeout(() => this.#sendPing(), delay);
				clearInterval(this.#ping);
				this.#ping = setInterval(() => this.#sendPing(), PING_INTERVAL);
				// A new participant, with a new snapshot to come.
				this.#statuses = {};
				if (this.#status) this.#write({ type: 'status', ...this.#status });
				break;
			case 'state':
				if (!this.room || message.room.revision >= this.room.revision) {
					this.room = this.#withStatuses(message.room);
				}
				break;
			case 'status':
				this.#applyStatus(message.participant, {
					sync: message.sync ?? null,
					place: message.place ?? null,
					revision: message.revision
				});
				break;
			case 'history':
				this.history = message.entries;
				break;
			case 'added':
				this.lastAdded = {
					added: message.added,
					skipped: message.skipped + this.#overflow,
					at: Date.now()
				};
				this.#overflow = 0;
				break;
			case 'pong':
				this.#clock.sample(message.t, localNow(), message.server_time);
				break;
			case 'error': {
				// A reorder lost to a newer change; that change's state shows the outcome.
				if (message.code === 'stale') break;
				const rejected = this.status !== 'open' && message.code === 'invalid';
				const final = FINAL_ERRORS[message.code] ?? (rejected ? 'rejected' : null);
				if (final) this.status = final;
				else this.lastError = { code: message.code, at: Date.now() };
				break;
			}
			case 'kicked':
				this.status = 'kicked';
				break;
			case 'closed':
				this.status = 'closed';
				break;
		}
	}
}
