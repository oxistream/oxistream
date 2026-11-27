import type { components } from '../api/schema';
import { ClockSync, backoffDelay, localNow } from './sync';

type Schemas = components['schemas'];
export type RoomState = Schemas['RoomState'];
export type RoomQueueEntry = Schemas['QueueEntry'];
export type RoomParticipant = Schemas['RoomParticipant'];
export type RoomErrorCode = Schemas['RoomErrorCode'];
export type RoomClientMessage = Schemas['RoomClientMessage'];
type ServerMessage = Schemas['RoomServerMessage'];

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
	participantId = $state<string | null>(null);
	isOwner = $state(false);
	/** The last error the server reported for something this client sent. */
	lastError = $state<{ code: RoomErrorCode; at: number } | null>(null);

	#roomId: string;
	#join: JoinDetails;
	#socket: WebSocket | undefined;
	#attempt = 0;
	#retry: ReturnType<typeof setTimeout> | undefined;
	#ping: ReturnType<typeof setInterval> | undefined;
	#clock = new ClockSync();
	#welcomeOffset = 0;
	#stopped = false;

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

	#write(message: RoomClientMessage) {
		if (this.#socket?.readyState === WebSocket.OPEN) this.#socket.send(JSON.stringify(message));
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
				break;
			case 'state':
				if (!this.room || message.room.revision >= this.room.revision) this.room = message.room;
				break;
			case 'pong':
				this.#clock.sample(message.t, localNow(), message.server_time);
				break;
			case 'error': {
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
