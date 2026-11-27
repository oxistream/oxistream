import { readFileSync } from 'node:fs';
import {
	expect,
	test,
	type Browser,
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
/** Videos with `sync` in the ID play the 60 s clip. */
const LONG = 'syncCCCCCC8';
const LONG_TOO = 'syncDDDDDDw';
const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

// The 6 s clip of the shorts tests, so the queue advances when it ends.
const CLIP = readFileSync(new URL('./fixtures/short.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/short.mpd', import.meta.url), 'utf8');
// A 60 s clip with a keyframe every second, long enough to watch the drift settle.
const LONG_CLIP = readFileSync(new URL('./fixtures/sync.webm', import.meta.url));
const LONG_MPD = readFileSync(new URL('./fixtures/sync.mpd', import.meta.url), 'utf8');
const isLong = (id: string) => id.startsWith('sync');

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

const TITLES: Record<string, string> = {
	[FIRST]: 'Fjord drive one',
	[SECOND]: 'Fjord drive two',
	[LONG]: 'Test pattern',
	[LONG_TOO]: 'Test pattern again'
};
const title = (id: string) => TITLES[id] ?? id;

function summary(id: string) {
	return {
		id,
		title: title(id),
		thumbnails: [],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: isLong(id) ? 60 : 6,
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
	at?: number;
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

	constructor(first = FIRST) {
		this.add(first, 'mara');
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

	setPosition(position: number, playing = this.playing, at = Date.now()) {
		this.position = position;
		this.positionAt = Math.min(Date.now(), Math.max(Date.now() - 5000, at));
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
				this.setPosition(message.position!, message.type === 'play', message.at);
				return this.broadcast();
			case 'seek':
				this.setPosition(message.position!, this.playing, message.at);
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
		const id = new URL(route.request().url()).pathname.split('/').at(-2)!;
		const clip = isLong(id) ? LONG_CLIP : CLIP;
		const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers()['range'] ?? '');
		if (!range) return route.fulfill({ status: 200, contentType: 'video/webm', body: clip });
		const start = Number(range[1]);
		const end = range[2] ? Number(range[2]) : clip.length - 1;
		return route.fulfill({
			status: 206,
			contentType: 'video/webm',
			headers: { 'Content-Range': `bytes ${start}-${end}/${clip.length}` },
			body: clip.subarray(start, end + 1)
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
			body: (isLong(id) ? LONG_MPD : MPD).replace('{id}', id)
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
const hasMetadata = (page: Page) =>
	media(page).evaluate((el: HTMLMediaElement) => el.readyState >= HTMLMediaElement.HAVE_METADATA);

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

/**
 * How far apart two players are, in seconds: both are read at nearly the same moment and
 * projected to a common instant with their playback rates.
 */
async function gap(a: Page, b: Page): Promise<number> {
	const read = (page: Page) =>
		media(page).evaluate((el: HTMLMediaElement) => ({
			at: performance.timeOrigin + performance.now(),
			position: el.currentTime,
			rate: el.playbackRate
		}));
	const [x, y] = await Promise.all([read(a), read(b)]);
	const at = Math.max(x.at, y.at);
	const project = (r: typeof x) => r.position + ((at - r.at) / 1000) * r.rate;
	return project(x) - project(y);
}

/** The largest gap over `seconds`, sampled twice a second. */
async function worstGap(a: Page, b: Page, seconds: number): Promise<number> {
	let worst = 0;
	for (let i = 0; i < seconds * 2; i++) {
		worst = Math.max(worst, Math.abs(await gap(a, b)));
		await a.waitForTimeout(500);
	}
	return worst;
}

const SETTLED_GAP = 0.05;

async function joinRoom(browser: Browser, room: FakeRoom, query = '') {
	const hostContext = await browser.newContext();
	const guestContext = await browser.newContext();
	await mockApi(hostContext, room, { signedIn: true });
	await mockApi(guestContext, room, { signedIn: false });
	const host = await hostContext.newPage();
	const guest = await guestContext.newPage();
	await host.goto(`/together/${ROOM}${query}`);
	await expect(host.getByTestId('room-status')).toHaveText('Connected');
	await guest.goto(`/together/${ROOM}${query}`);
	await guest.getByLabel('Your name').fill('Grace');
	await guest.getByRole('button', { name: 'Join', exact: true }).click();
	await expect(guest.getByTestId('room-status')).toHaveText('Connected');
	return { host, guest, hostContext, guestContext };
}

test('players stay within a few frames of each other', async ({ browser }) => {
	test.setTimeout(90_000);
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinRoom(browser, room, '?debug=sync');

	await expect.poll(() => hasMetadata(host)).toBe(true);
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect.poll(() => isPaused(guest)).toBe(false);
	const readout = guest.getByTestId('sync-readout');
	await expect(readout).toBeVisible();
	await expect(readout).toContainText('locked', { timeout: 15_000 });
	const settled = await worstGap(host, guest, 4);
	console.log(`steady gap: ${(settled * 1000).toFixed(1)} ms`);
	expect(settled).toBeLessThan(SETTLED_GAP);

	// A small offset is made up by playing slightly faster or slower, not by seeking.
	await media(guest).evaluate((el: HTMLMediaElement) => (el.currentTime += 0.4));
	await expect
		.poll(() => media(guest).evaluate((el: HTMLMediaElement) => el.playbackRate))
		.toBeLessThan(1);
	await expect
		.poll(async () => Math.abs(await gap(host, guest)), { timeout: 20_000 })
		.toBeLessThan(0.03);
	const caughtUp = await worstGap(host, guest, 3);
	console.log(`gap after catching up: ${(caughtUp * 1000).toFixed(1)} ms`);
	expect(caughtUp).toBeLessThan(SETTLED_GAP);
	// The speed used for catching up is not saved as the guest's preference.
	const saved = await guest.evaluate(() => localStorage.getItem('oxistream:player'));
	expect((JSON.parse(saved ?? '{}') as { playbackRate?: number }).playbackRate ?? 1).toBe(1);

	// After someone seeks, both land on the same spot again.
	await media(host).evaluate((el: HTMLMediaElement) => (el.currentTime = 30));
	await expect.poll(() => room.position).toBeGreaterThan(29);
	await expect
		.poll(async () => Math.abs(await gap(host, guest)), { timeout: 15_000 })
		.toBeLessThan(0.03);
	const afterSeek = await worstGap(host, guest, 3);
	console.log(`gap after a seek: ${(afterSeek * 1000).toFixed(1)} ms`);
	expect(afterSeek).toBeLessThan(SETTLED_GAP);

	// Paused, both show the same frame.
	await media(guest).evaluate((el: HTMLMediaElement) => el.pause());
	await expect.poll(() => isPaused(host)).toBe(true);
	await expect.poll(async () => Math.abs(await gap(host, guest))).toBeLessThan(0.04);

	await hostContext.close();
	await guestContext.close();
});

test('the room search lists more videos and other tabs add to the room', async ({ browser }) => {
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinRoom(browser, room);
	const video = (id: string, extra = {}) => ({ type: 'video', ...summary(id), ...extra });
	await guest.route('**/api/v0/search?*', (route) => {
		const url = new URL(route.request().url());
		if (url.searchParams.get('continuation') === 'page-2') {
			return json(route, { items: [video(SECOND)] });
		}
		return json(route, {
			items: [
				video(FIRST),
				video('shortEEEEEA', { title: 'A short', is_short: true }),
				video('liveFFFFFFE', { title: 'Live now', is_live: true }),
				{
					type: 'channel',
					id: 'UCfjord',
					name: 'Fjord channel',
					avatar: [],
					verified: false,
					description: ''
				}
			],
			continuation: 'page-2'
		});
	});
	const add = guest.getByTestId('room-add');
	await guest.getByLabel('Add a video').fill('fjord');
	await guest.getByRole('button', { name: 'Add', exact: true }).click();
	const results = add.getByRole('list', { name: 'Search results' });
	await expect(results.getByRole('button', { name: `Add ${title(FIRST)}` })).toBeVisible();
	await expect(results.getByRole('button', { name: `Add ${title(SECOND)}` })).toBeVisible();
	await expect(results.getByRole('button')).toHaveCount(2);
	await results.getByRole('button', { name: `Add ${title(SECOND)}` }).click();
	await expect(host.getByTestId('room-queue').getByText(title(SECOND))).toBeVisible();
	await expect(results.getByRole('button', { name: `Add ${title(SECOND)}` })).toBeDisabled();

	// A second tab of the host offers adding the video it shows to the open room.
	const other = await hostContext.newPage();
	await other.goto(`/watch?v=${LONG_TOO}`);
	const addHere = other.getByTestId('add-to-room');
	await expect(addHere).toHaveText("Add to mara's room");
	await addHere.click();
	await expect(addHere).toHaveText("Added to mara's room");
	await expect(host.getByTestId('room-queue').getByText(title(LONG_TOO))).toBeVisible();

	// Once the room tab is gone, nothing is offered.
	await host.goto('about:blank');
	await other.reload();
	await expect(other.getByRole('heading', { name: title(LONG_TOO) })).toBeVisible();
	await expect(other.getByTestId('add-to-room')).toHaveCount(0);

	await hostContext.close();
	await guestContext.close();
});
