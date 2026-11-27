import { readFileSync } from 'node:fs';
import {
	expect,
	test,
	type BrowserContext,
	type Locator,
	type Page,
	type Route,
	type WebSocketRoute
} from '@playwright/test';

test.use({
	locale: 'en-US',
	launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] }
});

// Every video plays the 6 s clip, so the queue advances quickly.
const CLIP = readFileSync(new URL('./fixtures/short.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/short.mpd', import.meta.url), 'utf8');

const TITLES: Record<string, string> = {
	alpineAAAA0: 'Alpine pass',
	beachBBBBB4: 'Beach walk',
	canyonCCCC8: 'Canyon rim',
	deltaDDDDDw: 'Delta at dusk'
};
const [ALPINE, BEACH, CANYON, DELTA] = Object.keys(TITLES);
const title = (id: string) => TITLES[id] ?? id;
const ROOM = 'UXVldWUtcm9vbS0xMjM0NTY';
const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

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

/** Just enough of a party server: one member, a queue that advances when a video ends. */
class FakeRoom {
	queue: { id: number; video: ReturnType<typeof summary>; added_by: string }[] = [];
	current: number | null = null;
	playing = false;
	position = 0;
	revision = 0;
	nextEntry = 1;
	socket: WebSocketRoute | null = null;

	constructor(first: string) {
		this.add(first);
	}

	add(videoId: string) {
		const id = this.nextEntry++;
		this.queue.push({ id, video: summary(videoId), added_by: 'mara' });
		this.current ??= id;
	}

	broadcast() {
		this.revision++;
		const current = this.queue.find((e) => e.id === this.current) ?? null;
		const room = {
			id: ROOM,
			revision: this.revision,
			owner: 'mara',
			current,
			queue: this.queue.filter((e) => e !== current),
			playing: this.playing,
			position: this.position,
			position_at: Date.now(),
			participants: [{ id: 'p1', name: 'mara', is_owner: true, is_guest: false }]
		};
		this.socket?.send(JSON.stringify({ type: 'state', room }));
	}

	handle(socket: WebSocketRoute, raw: string) {
		const message = JSON.parse(raw) as {
			type: string;
			t?: number;
			position?: number;
			video_id?: string;
			entry?: number;
		};
		switch (message.type) {
			case 'join':
				this.socket = socket;
				socket.send(
					JSON.stringify({
						type: 'welcome',
						participant_id: 'p1',
						is_owner: true,
						server_time: Date.now()
					})
				);
				return this.broadcast();
			case 'ping':
				return socket.send(JSON.stringify({ type: 'pong', t: message.t, server_time: Date.now() }));
			case 'play':
			case 'pause':
			case 'seek':
				this.position = message.position!;
				if (message.type !== 'seek') this.playing = message.type === 'play';
				return this.broadcast();
			case 'add':
				this.add(message.video_id!);
				return this.broadcast();
			case 'ended':
				if (message.entry !== this.current) return;
				this.queue = this.queue.filter((e) => e.id !== message.entry);
				this.current = this.queue[0]?.id ?? null;
				this.position = 0;
				return this.broadcast();
		}
	}
}

async function mockApi(context: BrowserContext, { room }: { room?: FakeRoom } = {}) {
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
	await context.route('**/api/v0/auth/me', (route) =>
		room ? json(route, account) : json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await context.route('**/api/v0/subscriptions*', (route) => json(route, []));
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
			related: Object.keys(TITLES)
				.filter((other) => other !== id)
				.map(summary)
		});
	});
	if (!room) return;
	await context.route(`**/api/v0/rooms/${ROOM}`, (route) =>
		json(route, { id: ROOM, owner: 'mara', is_owner: true, created_at: '2026-11-26T10:00:00Z' })
	);
	await context.routeWebSocket(/\/api\/v0\/rooms\/[^/]+\/ws$/, (socket) => {
		socket.onMessage((message) => room.handle(socket, String(message)));
	});
}

const watchUrl = (id: string) => new RegExp(`/watch\\?v=${id}$`);
const media = (page: Page) => page.locator('.app-player dash-video');

async function videoMenu(page: Page, id: string) {
	await page
		.locator('section[aria-labelledby="related-heading"]')
		.getByRole('button', { name: `More actions for ${title(id)}` })
		.click();
	return page.getByRole('menu');
}

async function queueVideo(page: Page, id: string, action: 'Add to queue' | 'Play next') {
	const menu = await videoMenu(page, id);
	await menu.getByRole('menuitem', { name: action }).click();
	await expect(menu).toBeHidden();
}

const titles = (list: Locator) => list.getByTestId('queue-entry-title').allInnerTexts();

async function entryAction(list: Locator, id: string, action: string) {
	await list.getByRole('button', { name: `Actions for ${title(id)}` }).click();
	await list.page().getByRole('menuitem', { name: action }).click();
}

test('queued videos play in order after the current one', async ({ context, page }) => {
	await mockApi(context);
	await page.goto(`/watch?v=${ALPINE}`);
	await expect(page.getByRole('heading', { level: 1, name: title(ALPINE) })).toBeVisible();
	await expect(page.getByTestId('up-next')).toHaveCount(0);

	await queueVideo(page, BEACH, 'Add to queue');
	await queueVideo(page, CANYON, 'Add to queue');
	await queueVideo(page, DELTA, 'Play next');
	const upNext = page.getByTestId('up-next');
	await expect(upNext).toBeVisible();
	await expect.poll(() => titles(upNext)).toEqual([title(DELTA), title(BEACH), title(CANYON)]);

	// The 6 s clip ends and each queued video follows, taken off the queue as it starts.
	await expect(page).toHaveURL(watchUrl(DELTA), { timeout: 20_000 });
	await expect(page.getByRole('heading', { level: 1, name: title(DELTA) })).toBeVisible();
	await expect.poll(() => titles(upNext)).toEqual([title(BEACH), title(CANYON)]);
	await expect(page).toHaveURL(watchUrl(BEACH), { timeout: 20_000 });
	await expect(page).toHaveURL(watchUrl(CANYON), { timeout: 20_000 });
	await expect(page.getByTestId('up-next')).toHaveCount(0);
	await expect
		.poll(() => media(page).evaluate((el: HTMLMediaElement) => el.ended), {
			timeout: 20_000
		})
		.toBe(true);
	await expect(page).toHaveURL(watchUrl(CANYON));
});

test('the queue can be reordered, trimmed and cleared, and lasts the tab', async ({
	context,
	page
}) => {
	await mockApi(context);
	await page.goto(`/watch?v=${ALPINE}`);
	await expect(page.getByRole('heading', { level: 1, name: title(ALPINE) })).toBeVisible();
	await media(page).evaluate((el: HTMLMediaElement) => el.pause());
	for (const id of [BEACH, CANYON, DELTA]) await queueVideo(page, id, 'Add to queue');
	const upNext = page.getByTestId('up-next');
	await expect.poll(() => titles(upNext)).toEqual([title(BEACH), title(CANYON), title(DELTA)]);

	await entryAction(upNext, DELTA, 'Move up');
	await expect.poll(() => titles(upNext)).toEqual([title(BEACH), title(DELTA), title(CANYON)]);
	await entryAction(upNext, CANYON, 'Move to top');
	await expect.poll(() => titles(upNext)).toEqual([title(CANYON), title(BEACH), title(DELTA)]);
	await entryAction(upNext, CANYON, 'Move down');
	await expect.poll(() => titles(upNext)).toEqual([title(BEACH), title(CANYON), title(DELTA)]);
	await entryAction(upNext, BEACH, 'Remove');
	await expect.poll(() => titles(upNext)).toEqual([title(CANYON), title(DELTA)]);

	await page.reload();
	await expect.poll(() => titles(upNext)).toEqual([title(CANYON), title(DELTA)]);

	// Playing one now takes it off the queue.
	await upNext.getByRole('button', { name: title(DELTA), exact: true }).click();
	await expect(page).toHaveURL(watchUrl(DELTA));
	await expect.poll(() => titles(upNext)).toEqual([title(CANYON)]);

	await upNext.getByRole('button', { name: 'Clear queue' }).click();
	await expect(page.getByTestId('up-next')).toHaveCount(0);
});

test('the mini player shows the queue, skips ahead and plays on by itself', async ({
	context,
	page
}) => {
	await mockApi(context);
	await page.goto(`/watch?v=${ALPINE}`);
	await expect(page.getByRole('heading', { level: 1, name: title(ALPINE) })).toBeVisible();
	await media(page).evaluate((el: HTMLMediaElement) => el.pause());
	for (const id of [BEACH, CANYON, DELTA]) await queueVideo(page, id, 'Add to queue');

	await page.getByRole('link', { name: 'Home', exact: true }).filter({ visible: true }).click();
	const mini = page.getByRole('region', { name: 'Mini player' });
	await expect(mini.getByTestId('mini-player-title')).toHaveText(title(ALPINE));
	await mini.getByRole('button', { name: 'Show queue' }).click();
	const list = mini.getByTestId('up-next');
	await expect.poll(() => titles(list)).toEqual([title(BEACH), title(CANYON), title(DELTA)]);
	await entryAction(list, DELTA, 'Remove');
	await expect.poll(() => titles(list)).toEqual([title(BEACH), title(CANYON)]);

	await mini.getByRole('button', { name: 'Next video' }).click();
	await expect(mini.getByTestId('mini-player-title')).toHaveText(title(BEACH));
	await expect(page).toHaveURL('/');
	await expect.poll(() => titles(list)).toEqual([title(CANYON)]);

	// The last one follows when the clip ends, still in the mini player.
	await expect(mini.getByTestId('mini-player-title')).toHaveText(title(CANYON), {
		timeout: 20_000
	});
	await expect(mini.getByRole('button', { name: 'Next video' })).toHaveCount(0);
	await expect(mini.getByRole('button', { name: 'Show queue' })).toHaveCount(0);
	await expect(page).toHaveURL('/');
});

test('the queue opens above the mini player on phones', async ({ context, page }) => {
	await page.setViewportSize({ width: 390, height: 844 });
	await mockApi(context);
	await page.goto(`/watch?v=${ALPINE}`);
	await expect(page.getByRole('heading', { level: 1, name: title(ALPINE) })).toBeVisible();
	await media(page).evaluate((el: HTMLMediaElement) => el.pause());
	for (const id of [BEACH, CANYON]) await queueVideo(page, id, 'Add to queue');
	await expect
		.poll(() => titles(page.getByTestId('up-next')))
		.toEqual([title(BEACH), title(CANYON)]);

	await page.getByRole('link', { name: 'Home', exact: true }).filter({ visible: true }).click();
	const mini = page.getByRole('region', { name: 'Mini player' });
	await mini.getByRole('button', { name: 'Show queue' }).click();
	const list = mini.getByTestId('up-next');
	await expect(list).toBeVisible();
	const box = (await mini.boundingBox())!;
	expect(box.x).toBeGreaterThanOrEqual(0);
	expect(box.x + box.width).toBeLessThanOrEqual(390);
	expect(box.y).toBeGreaterThan(0);
	expect(box.y + box.height).toBeLessThanOrEqual(844);
	const listBox = (await list.boundingBox())!;
	expect(listBox.width).toBeGreaterThan(box.width - 4);

	await mini.getByRole('button', { name: 'Hide queue' }).click();
	await expect(list).toHaveCount(0);
});

test('a party keeps its own queue', async ({ context, page }) => {
	test.slow();
	const room = new FakeRoom(BEACH);
	await mockApi(context, { room });
	await page.goto(`/watch?v=${ALPINE}`);
	await expect(page.getByRole('heading', { level: 1, name: title(ALPINE) })).toBeVisible();
	await media(page).evaluate((el: HTMLMediaElement) => el.pause());
	await queueVideo(page, DELTA, 'Add to queue');

	await page.goto(`/party/${ROOM}`);
	await expect(page).toHaveURL(watchUrl(BEACH));
	await expect(page.getByRole('heading', { level: 1, name: title(BEACH) })).toBeVisible();
	await expect(page.getByTestId('up-next')).toHaveCount(0);

	const menu = await videoMenu(page, CANYON);
	await expect(menu.getByRole('menuitem', { name: 'Play next' })).toHaveCount(0);
	await expect(menu.getByRole('menuitem', { name: 'Add to queue' })).toHaveCount(0);
	await menu.getByRole('menuitem', { name: 'Add to party' }).click();
	await expect.poll(() => room.queue.map((e) => e.video.id)).toEqual([BEACH, CANYON]);

	// The party's queue follows, not this tab's.
	await expect
		.poll(() => media(page).evaluate((el: HTMLMediaElement) => el.readyState))
		.toBeGreaterThanOrEqual(1);
	room.playing = true;
	room.broadcast();
	await expect(page).toHaveURL(watchUrl(CANYON), { timeout: 20_000 });
	await expect.poll(() => room.current).toBe(2);
	const stored = await page.evaluate(() => sessionStorage.getItem('oxistream:queue'));
	const entries = JSON.parse(stored ?? '[]') as { id: string }[];
	expect(entries.map((e) => e.id)).toEqual([DELTA]);
});
