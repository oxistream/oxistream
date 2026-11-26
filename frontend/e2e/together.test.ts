import { readFileSync } from 'node:fs';
import {
	expect,
	test,
	type BrowserContext,
	type Page,
	type Route,
	type WebSocketRoute
} from '@playwright/test';

test.use({
	locale: 'en-US',
	launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] }
});

const ROOM = 'Qm9vbS1vbmUtMTIzNDU2Nw';
const FIRST = 'fjordAAAAA0';
const SECOND = 'fjordBBBBB4';
const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

// The 6 s clip of the shorts tests, so the queue advances when it ends.
const CLIP = readFileSync(new URL('./fixtures/short.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/short.mpd', import.meta.url), 'utf8');

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

const title = (id: string) => (id === FIRST ? 'Fjord drive one' : 'Fjord drive two');

function summary(id: string) {
	return {
		id,
		title: title(id),
		thumbnails: [],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: 6,
		view_count: null,
		viewers: null,
		published_at: null,
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false,
		description: null
	};
}

type ClientMessage = {
	type: string;
	name?: string;
	t?: number;
	position?: number;
	video_id?: string;
	entry?: number;
};

/** A room server in the test process, shared by both browsers. */
class FakeRoom {
	sockets = new Map<WebSocketRoute, { id: string; name: string; owner: boolean }>();
	queue: { id: number; video: ReturnType<typeof summary>; added_by: string }[] = [];
	current: number | null = null;
	playing = false;
	position = 0;
	positionAt = Date.now();
	revision = 0;
	nextEntry = 1;

	constructor() {
		this.add(FIRST, 'mara');
	}

	add(videoId: string, by: string) {
		const id = this.nextEntry++;
		this.queue.push({ id, video: summary(videoId), added_by: by });
		this.current ??= id;
	}

	state() {
		const current = this.queue.find((e) => e.id === this.current) ?? null;
		return {
			id: ROOM,
			revision: this.revision,
			owner: 'mara',
			current,
			queue: this.queue.filter((e) => e !== current),
			playing: this.playing,
			position: this.position,
			position_at: this.positionAt,
			participants: [...this.sockets.values()].map((p) => ({
				id: p.id,
				name: p.name,
				is_owner: p.owner,
				is_guest: !p.owner
			}))
		};
	}

	broadcast() {
		this.revision++;
		const message = JSON.stringify({ type: 'state', room: this.state() });
		for (const socket of this.sockets.keys()) socket.send(message);
	}

	setPosition(position: number, playing = this.playing) {
		this.position = position;
		this.positionAt = Date.now();
		this.playing = playing;
	}

	handle(socket: WebSocketRoute, owner: boolean, raw: string) {
		const message = JSON.parse(raw) as ClientMessage;
		switch (message.type) {
			case 'join': {
				const id = `p${this.sockets.size + 1}`;
				this.sockets.set(socket, { id, name: owner ? 'mara' : message.name!, owner });
				const at = Date.now();
				socket.send(
					JSON.stringify({ type: 'welcome', participant_id: id, is_owner: owner, server_time: at })
				);
				return this.broadcast();
			}
			case 'ping':
				return socket.send(JSON.stringify({ type: 'pong', t: message.t, server_time: Date.now() }));
			case 'play':
			case 'pause':
				this.setPosition(message.position!, message.type === 'play');
				return this.broadcast();
			case 'seek':
				this.setPosition(message.position!);
				return this.broadcast();
			case 'add':
				this.add(message.video_id!, this.sockets.get(socket)!.name);
				return this.broadcast();
			case 'ended':
			case 'skip':
				if (message.entry !== this.current) return;
				this.queue = this.queue.filter((e) => e.id !== message.entry);
				this.current = this.queue[0]?.id ?? null;
				this.setPosition(0, this.current !== null);
				return this.broadcast();
		}
	}
}

async function mockApi(
	context: BrowserContext,
	room: FakeRoom,
	{ signedIn }: { signedIn: boolean }
) {
	await context.route('**/proxy/**', (route) => route.fulfill({ status: 404, body: '' }));
	await context.route('**/proxy/stream/**', (route) => {
		const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers()['range'] ?? '');
		if (!range) return route.fulfill({ status: 200, contentType: 'video/webm', body: CLIP });
		const start = Number(range[1]);
		const end = range[2] ? Number(range[2]) : CLIP.length - 1;
		return route.fulfill({
			status: 206,
			contentType: 'video/webm',
			headers: { 'Content-Range': `bytes ${start}-${end}/${CLIP.length}` },
			body: CLIP.subarray(start, end + 1)
		});
	});
	await context.route('**/api/v0/**', (route) =>
		json(route, { code: 'not_found', message: '' }, 404)
	);
	await context.route('**/api/v0/instance', (route) =>
		json(route, {
			name: 'Oxistream',
			version: '0.1.0',
			auth: {
				local_enabled: true,
				registration_enabled: true,
				require_login: false,
				oidc_enabled: false,
				oidc_name: null
			}
		})
	);
	await context.route('**/api/v0/auth/me', (route) =>
		signedIn ? json(route, account) : json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await context.route('**/api/v0/subscriptions*', (route) => json(route, []));
	await context.route(`**/api/v0/rooms/${ROOM}`, (route) =>
		json(route, { id: ROOM, owner: 'mara', is_owner: signedIn, created_at: '2026-11-26T10:00:00Z' })
	);
	await context.route('**/api/v0/videos/*/manifest.mpd', (route) => {
		const id = new URL(route.request().url()).pathname.split('/').at(-2)!;
		return route.fulfill({
			status: 200,
			contentType: 'application/dash+xml',
			body: MPD.replace('{id}', id)
		});
	});
	await context.route(/\/api\/v0\/videos\/[^/]+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop()!;
		return json(route, {
			...summary(id),
			description: '',
			like_count: null,
			keywords: [],
			subtitles: [],
			related: []
		});
	});
	await context.routeWebSocket(/\/api\/v0\/rooms\/[^/]+\/ws$/, (socket) => {
		socket.onMessage((message) => room.handle(socket, signedIn, String(message)));
		socket.onClose(() => {
			room.sockets.delete(socket);
			room.broadcast();
		});
	});
}

const media = (page: Page) => page.getByTestId('room-player').locator('dash-video');
const isPaused = (page: Page) => media(page).evaluate((el: HTMLMediaElement) => el.paused);

test('two people stay in sync and the queue plays in order', async ({ browser }) => {
	const room = new FakeRoom();
	const hostContext = await browser.newContext();
	const guestContext = await browser.newContext();
	await mockApi(hostContext, room, { signedIn: true });
	await mockApi(guestContext, room, { signedIn: false });
	const host = await hostContext.newPage();
	const guest = await guestContext.newPage();

	await host.goto(`/together/${ROOM}`);
	await expect(host.getByTestId('room-status')).toHaveText('Connected');
	await expect(host.getByRole('button', { name: 'Close room' })).toBeVisible();

	await guest.goto(`/together/${ROOM}`);
	await expect(guest.getByRole('heading', { name: "Join mara's room" })).toBeVisible();
	await guest.getByLabel('Your name').fill('Grace');
	await guest.getByRole('button', { name: 'Join', exact: true }).click();
	await expect(guest.getByTestId('room-status')).toHaveText('Connected');

	const people = host.getByTestId('room-participants');
	await expect(people.getByText('Grace')).toBeVisible();
	await expect(people.getByRole('button', { name: 'Remove Grace' })).toBeVisible();
	await expect(guest.getByTestId('room-participants').getByRole('button')).toHaveCount(0);
	await expect(guest.getByRole('heading', { name: title(FIRST) })).toBeVisible();

	// Playing in one browser plays in the other; so does pausing.
	await expect.poll(() => isPaused(guest)).toBe(true);
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect.poll(() => room.playing).toBe(true);
	await expect.poll(() => isPaused(guest)).toBe(false);

	await media(guest).evaluate((el: HTMLMediaElement) => el.pause());
	await expect.poll(() => room.playing).toBe(false);
	await expect.poll(() => isPaused(host)).toBe(true);
	const positions = await Promise.all(
		[host, guest].map((page) => media(page).evaluate((el: HTMLMediaElement) => el.currentTime))
	);
	expect(Math.abs(positions[0] - positions[1])).toBeLessThan(0.5);

	// A video queued by the guest shows up for the host and plays once the first one ends.
	await guest.getByLabel('Add a video').fill(`https://youtu.be/${SECOND}`);
	await guest.getByRole('button', { name: 'Add', exact: true }).click();
	const queue = host.getByTestId('room-queue');
	await expect(queue.getByText(title(SECOND))).toBeVisible();
	await expect(queue.getByText('Added by Grace')).toBeVisible();

	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect(host.getByRole('heading', { name: title(SECOND) })).toBeVisible({ timeout: 15_000 });
	await expect(guest.getByRole('heading', { name: title(SECOND) })).toBeVisible();
	await expect.poll(() => isPaused(guest)).toBe(false);

	await hostContext.close();
	await guestContext.close();
});
