import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RoomConnection, socketUrl } from './connection.svelte';

class FakeSocket extends EventTarget {
	static OPEN = 1;
	static instances: FakeSocket[] = [];
	readyState = 0;
	sent: unknown[] = [];

	constructor(readonly url: string) {
		super();
		FakeSocket.instances.push(this);
	}

	send(data: string) {
		this.sent.push(JSON.parse(data));
	}

	close() {
		this.readyState = 3;
		this.dispatchEvent(new Event('close'));
	}

	open() {
		this.readyState = 1;
		this.dispatchEvent(new Event('open'));
	}

	receive(message: unknown) {
		this.dispatchEvent(new MessageEvent('message', { data: JSON.stringify(message) }));
	}
}

const room = (revision: number) => ({
	id: 'room',
	revision,
	owner: 'mara',
	current: null,
	queue: [],
	playing: false,
	position: 0,
	position_at: 0,
	participants: []
});

const welcome = { type: 'welcome', participant_id: 'p1', is_owner: false, server_time: 0 };

beforeEach(() => {
	FakeSocket.instances = [];
	vi.useFakeTimers();
	vi.stubGlobal('WebSocket', FakeSocket);
	vi.stubGlobal('window', { location: { protocol: 'https:', host: 'tube.example' } });
});

afterEach(() => {
	vi.useRealTimers();
	vi.unstubAllGlobals();
});

describe('socketUrl', () => {
	it('follows the page scheme', () => {
		const http = { protocol: 'http:', host: 'localhost:5173' } as Location;
		expect(socketUrl('a b', http)).toBe('ws://localhost:5173/api/v0/rooms/a%20b/ws');
		expect(socketUrl('x')).toBe('wss://tube.example/api/v0/rooms/x/ws');
	});
});

describe('RoomConnection', () => {
	it('joins, keeps the newest state and pings the server clock', () => {
		const conn = new RoomConnection('room', { name: 'Grace', guestKey: 'k'.repeat(16) });
		conn.connect();
		const socket = FakeSocket.instances[0];
		socket.open();
		expect(socket.sent[0]).toEqual({ type: 'join', name: 'Grace', guest_key: 'k'.repeat(16) });

		socket.receive(welcome);
		expect(conn.status).toBe('open');
		expect(conn.participantId).toBe('p1');
		socket.receive({ type: 'state', room: room(3) });
		socket.receive({ type: 'state', room: room(2) });
		expect(conn.room?.revision).toBe(3);

		vi.advanceTimersByTime(0);
		expect(socket.sent).toContainEqual(expect.objectContaining({ type: 'ping' }));
		conn.disconnect();
	});

	it('reconnects after a dropped connection but not after a kick', () => {
		const conn = new RoomConnection('room', {});
		conn.connect();
		FakeSocket.instances[0].open();
		FakeSocket.instances[0].receive(welcome);
		FakeSocket.instances[0].close();
		expect(conn.status).toBe('reconnecting');
		vi.advanceTimersByTime(1500);
		expect(FakeSocket.instances).toHaveLength(2);

		const socket = FakeSocket.instances[1];
		socket.open();
		socket.receive(welcome);
		socket.receive({ type: 'kicked' });
		socket.close();
		vi.advanceTimersByTime(60_000);
		expect(conn.status).toBe('kicked');
		expect(FakeSocket.instances).toHaveLength(2);
	});

	it('treats a missing room as final and other errors as notices', () => {
		const conn = new RoomConnection('room', {});
		conn.connect();
		const socket = FakeSocket.instances[0];
		socket.open();
		socket.receive(welcome);
		socket.receive({ type: 'error', code: 'queue_full' });
		expect(conn.status).toBe('open');
		expect(conn.lastError?.code).toBe('queue_full');

		const other = new RoomConnection('gone', {});
		other.connect();
		FakeSocket.instances[1].open();
		FakeSocket.instances[1].receive({ type: 'error', code: 'not_found' });
		expect(other.status).toBe('not_found');
		expect(other.final).toBe(true);
		conn.disconnect();
	});
});
