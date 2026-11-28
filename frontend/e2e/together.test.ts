import { readFileSync } from 'node:fs';
import jsQR from 'jsqr';
import {
	expect,
	test,
	type Browser,
	type BrowserContext,
	type Locator,
	type Page,
	type Route,
	type WebSocketRoute
} from '@playwright/test';

test.use({
	locale: 'en-US',
	// Only Chromium knows this flag; WebKit on Linux refuses to start with it.
	launchOptions: [
		async ({ browserName }, use) =>
			use(
				browserName === 'chromium' ? { args: ['--autoplay-policy=no-user-gesture-required'] } : {}
			),
		{ scope: 'worker' }
	]
});

const ROOM = 'Qm9vbS1vbmUtMTIzNDU2Nw';
const FIRST = 'fjordAAAAA0';
const SECOND = 'fjordBBBBB4';
/** Videos with `sync` in the ID play the 60 s clip. */
const LONG = 'syncCCCCCC8';
const LONG_TOO = 'syncDDDDDDw';
/** Videos with `live` in the ID are livestreams, which cannot play. */
const LIVE = 'liveEEEEEE0';
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
	[LONG_TOO]: 'Test pattern again',
	[LIVE]: 'Live from the fjord'
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
		is_live: id.startsWith('live'),
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
	play?: boolean;
	entry?: number;
	video_ids?: string[];
	to?: number;
	revision?: number;
	enabled?: boolean;
	sync?: string | null;
	place?: string;
	text?: string;
	reaction?: string;
	control?: string;
	add?: string;
};

type FakeParticipant = {
	id: string;
	name: string;
	owner: boolean;
	sync: string | null;
	place: string | null;
	statusRevision: number;
};

type FakeEntry = {
	id: number;
	video: ReturnType<typeof summary>;
	added_by: string;
	autoplay: boolean;
	/** Who queued it, see {@link FakeRoom.key}. */
	key: string;
};

type FakeMessage = { id: number; name: string; key: string; owner: boolean; text: string };

/** A party server in the test process, shared by both browsers. */
class FakeRoom {
	sockets = new Map<WebSocketRoute, FakeParticipant>();
	joins = 0;
	queue: FakeEntry[] = [];
	/** Played entries, newest first, one per video. */
	history: FakeEntry[] = [];
	historyChanged = false;
	current: number | null = null;
	autoplay = false;
	/** Every video queued so far, which autoplay does not repeat. */
	seen = new Set<string>();
	playing = false;
	position = 0;
	positionAt = Date.now();
	revision = 0;
	nextEntry = 1;
	permissions = { control: 'everyone', add: 'everyone' };
	chat: FakeMessage[] = [];
	/** Every message the room received, by sender. */
	received: { owner: boolean; type: string }[] = [];

	constructor(first = FIRST) {
		this.add(first, 'mara');
	}

	/** Stands in for the server's participant key: one per account or guest name. */
	key(socket: WebSocketRoute) {
		const person = this.sockets.get(socket);
		return person?.owner ? 'host' : `guest:${person?.name}`;
	}

	add(videoId: string, by: string, play = false, autoplay = false, key = '') {
		const id = this.nextEntry++;
		this.seen.add(videoId);
		this.queue.push({ id, video: summary(videoId), added_by: autoplay ? '' : by, autoplay, key });
		if (play) {
			this.retire(this.current);
			this.current = id;
			this.setPosition(0, true);
		}
		this.current ??= id;
	}

	retire(entry: number | null) {
		const played = this.queue.find((e) => e.id === entry);
		if (!played) return;
		this.queue = this.queue.filter((e) => e !== played);
		this.history = [played, ...this.history.filter((e) => e.video.id !== played.video.id)];
		this.historyChanged = true;
	}

	upcoming() {
		return this.queue.filter((e) => e.id !== this.current);
	}

	/** What autoplay appends once the current entry, the last one, ends: the first unseen title. */
	upNext() {
		if (!this.autoplay || this.current === null || this.upcoming().length > 0) return null;
		return Object.keys(TITLES).find((id) => !this.seen.has(id) && !id.startsWith('live')) ?? null;
	}

	reorder(upcoming: typeof this.queue) {
		this.queue = [...this.queue.filter((e) => e.id === this.current), ...upcoming];
	}

	/** As seen by `viewer`, who owns the entries it queued. */
	state(viewer?: string) {
		const entry = ({ key, ...e }: FakeEntry) => ({ ...e, own: key !== '' && key === viewer });
		const current = this.queue.find((e) => e.id === this.current) ?? null;
		return {
			id: ROOM,
			revision: this.revision,
			owner: 'mara',
			current: current && entry(current),
			queue: this.queue.filter((e) => e !== current).map(entry),
			permissions: this.permissions,
			playing: this.playing,
			position: this.position,
			position_at: this.positionAt,
			autoplay: this.autoplay,
			up_next: this.upNext() ? summary(this.upNext()!) : null,
			participants: [...this.sockets.values()].map((p) => ({
				id: p.id,
				name: p.name,
				is_owner: p.owner,
				is_guest: !p.owner,
				sync: p.sync,
				place: p.place,
				status_revision: p.statusRevision
			}))
		};
	}

	broadcast() {
		this.revision++;
		const history = JSON.stringify({
			type: 'history',
			entries: this.history.map((e) => ({ ...e, key: undefined, own: false }))
		});
		for (const socket of this.sockets.keys()) {
			if (this.historyChanged) socket.send(history);
			socket.send(JSON.stringify({ type: 'state', room: this.state(this.key(socket)) }));
		}
		this.historyChanged = false;
	}

	chatMessage(message: FakeMessage, viewer: string) {
		return {
			id: message.id,
			name: message.name,
			is_owner: message.owner,
			is_guest: !message.owner,
			own: message.key === viewer,
			text: message.text,
			sent_at: Date.now()
		};
	}

	/** What the host's permissions keep others from doing. */
	forbidden(owner: boolean, message: ClientMessage) {
		if (owner) return false;
		if (message.type === 'permissions') return true;
		const control = this.permissions.control === 'everyone';
		const add = this.permissions.add === 'everyone';
		switch (message.type) {
			case 'add':
				return !add || (message.play === true && !control);
			case 'add_many':
				return !add;
			case 'play':
			case 'pause':
			case 'seek':
			case 'select':
			case 'skip':
			case 'move':
			case 'shuffle':
			case 'autoplay':
				return !control;
			default:
				return false;
		}
	}

	setPosition(position: number, playing = this.playing, at = Date.now()) {
		this.position = position;
		this.positionAt = Math.min(Date.now(), Math.max(Date.now() - 5000, at));
		this.playing = playing;
	}

	handle(socket: WebSocketRoute, owner: boolean, raw: string) {
		const message = JSON.parse(raw) as ClientMessage;
		this.received.push({ owner, type: message.type });
		if (this.forbidden(owner, message)) {
			return socket.send(JSON.stringify({ type: 'error', code: 'forbidden' }));
		}
		switch (message.type) {
			case 'join': {
				const id = `p${++this.joins}`;
				const name = owner ? 'mara' : message.name!;
				this.sockets.set(socket, { id, name, owner, sync: null, place: null, statusRevision: 0 });
				const at = Date.now();
				socket.send(
					JSON.stringify({ type: 'welcome', participant_id: id, is_owner: owner, server_time: at })
				);
				socket.send(JSON.stringify({ type: 'history', entries: this.history }));
				const viewer = this.key(socket);
				const messages = this.chat.map((m) => this.chatMessage(m, viewer));
				socket.send(JSON.stringify({ type: 'chat_history', messages }));
				return this.broadcast();
			}
			case 'chat': {
				const person = this.sockets.get(socket)!;
				const text = message.text!.trim().replace(/\s+/g, ' ');
				const key = this.key(socket);
				const sent = { id: this.chat.length + 1, name: person.name, owner, key, text };
				this.chat.push(sent);
				for (const other of this.sockets.keys()) {
					const chat = this.chatMessage(sent, this.key(other));
					other.send(JSON.stringify({ type: 'chat', message: chat }));
				}
				return;
			}
			case 'react': {
				const participant = this.sockets.get(socket)!.id;
				const reaction = JSON.stringify({
					type: 'reaction',
					participant,
					reaction: message.reaction
				});
				for (const other of this.sockets.keys()) other.send(reaction);
				return;
			}
			case 'permissions':
				this.permissions = {
					control: message.control ?? this.permissions.control,
					add: message.add ?? this.permissions.add
				};
				return this.broadcast();
			case 'ping':
				return socket.send(JSON.stringify({ type: 'pong', t: message.t, server_time: Date.now() }));
			case 'status': {
				// Passed on without a new revision, like the real server.
				const person = this.sockets.get(socket);
				if (!person) return;
				person.sync = message.sync ?? null;
				person.place = message.place ?? null;
				person.statusRevision++;
				const status = JSON.stringify({
					type: 'status',
					participant: person.id,
					sync: person.sync,
					place: person.place,
					revision: person.statusRevision
				});
				for (const other of this.sockets.keys()) other.send(status);
				return;
			}
			case 'play':
			case 'pause':
				this.setPosition(message.position!, message.type === 'play', message.at);
				return this.broadcast();
			case 'seek':
				this.setPosition(message.position!, this.playing, message.at);
				return this.broadcast();
			case 'add':
				this.add(
					message.video_id!,
					this.sockets.get(socket)!.name,
					message.play,
					false,
					this.key(socket)
				);
				return this.broadcast();
			case 'select': {
				if (!this.queue.some((e) => e.id === message.entry)) return;
				this.retire(this.current);
				this.current = message.entry!;
				this.setPosition(0, this.playing);
				return this.broadcast();
			}
			case 'add_many': {
				// Livestreams cannot be queued, as on the real server.
				const ids = message.video_ids!.filter((id) => !id.startsWith('live'));
				for (const id of ids) this.add(id, this.sockets.get(socket)!.name);
				const skipped = message.video_ids!.length - ids.length;
				socket.send(JSON.stringify({ type: 'added', added: ids.length, skipped }));
				return this.broadcast();
			}
			case 'move': {
				if (message.revision !== this.revision) {
					return socket.send(JSON.stringify({ type: 'error', code: 'stale' }));
				}
				const upcoming = this.upcoming();
				const from = upcoming.findIndex((e) => e.id === message.entry);
				if (from < 0) return;
				const [moved] = upcoming.splice(from, 1);
				upcoming.splice(Math.min(message.to!, upcoming.length), 0, moved);
				this.reorder(upcoming);
				return this.broadcast();
			}
			case 'shuffle':
				// Reversed, so the test knows the outcome.
				this.reorder(this.upcoming().reverse());
				return this.broadcast();
			case 'autoplay':
				this.autoplay = message.enabled!;
				return this.broadcast();
			case 'remove':
				this.queue = this.queue.filter((e) => e.id !== message.entry || e.id === this.current);
				return this.broadcast();
			case 'ended':
			case 'skip':
				if (message.entry !== this.current) return;
				{
					const next = this.upNext();
					this.retire(message.entry);
					this.current = this.queue[0]?.id ?? null;
					if (!this.current && next) this.add(next, '', false, true);
				}
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
			related: Object.keys(TITLES)
				.filter((other) => other !== id)
				.map(summary)
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

/** The party's video, in the watch page slot or the mini player. */
const media = (page: Page) => page.locator('.app-player dash-video');
const isPaused = (page: Page) => media(page).evaluate((el: HTMLMediaElement) => el.paused);
const hasMetadata = (page: Page) =>
	media(page).evaluate((el: HTMLMediaElement) => el.readyState >= HTMLMediaElement.HAVE_METADATA);
const watchUrl = (id: string) => new RegExp(`/watch\\?v=${id}$`);

async function openParty(page: Page, query = '') {
	await page.goto(`/party/${ROOM}${query}`);
}

async function joinAsGuest(page: Page, query = '') {
	await openParty(page, query);
	await expect(page.getByRole('heading', { name: "Join mara's party" })).toBeVisible();
	await page.getByLabel('Your name').fill('Grace');
	await page.getByRole('button', { name: 'Join', exact: true }).click();
}

/** The party sidebar, or the sheet on narrow screens. */
async function openPanel(page: Page) {
	const panel = page.getByTestId('party-panel');
	if (await panel.isVisible()) return panel;
	await page.getByTestId('party-button').click();
	await expect(panel).toBeVisible();
	return panel;
}

/** Opens the party menu in the panel and returns its item `name`. */
async function partyMenu(page: Page, name: string) {
	await page.getByTestId('party-panel').getByTestId('party-menu').click();
	return page.getByRole('menuitem', { name });
}

async function closePanel(page: Page) {
	const panel = page.getByTestId('party-panel');
	await panel.getByRole('button', { name: 'Close' }).click();
	await expect(panel).toBeHidden();
}

/** Follows a related video on the watch page, without reloading the app. */
async function browseTo(page: Page, id: string) {
	await page
		.locator('section[aria-labelledby="related-heading"]')
		.getByRole('link', { name: title(id) })
		.first()
		.click();
	await expect(page).toHaveURL(watchUrl(id));
}

test('two people stay in sync and the queue plays in order', async ({ browser }) => {
	const room = new FakeRoom();
	const hostContext = await browser.newContext();
	const guestContext = await browser.newContext();
	await mockApi(hostContext, room, { signedIn: true });
	await mockApi(guestContext, room, { signedIn: false });
	const host = await hostContext.newPage();
	const guest = await guestContext.newPage();

	// The invite link opens the watch page of what the party plays.
	await openParty(host);
	await expect(host).toHaveURL(watchUrl(FIRST));
	await expect(host.getByRole('heading', { name: title(FIRST) })).toBeVisible();
	await expect(host.getByTestId('party-count')).toHaveText('1');

	await joinAsGuest(guest);
	await expect(guest).toHaveURL(watchUrl(FIRST));
	await expect(host.getByTestId('party-count')).toHaveText('2');
	await expect(guest.getByTestId('party-count')).toHaveText('2');

	const hostPanel = await openPanel(host);
	await expect(hostPanel.getByTestId('party-status')).toHaveText('Connected');
	await expect(hostPanel.getByTestId('party-invite')).toHaveAttribute(
		'data-url',
		new RegExp(`/party/${ROOM}$`)
	);
	await expect(hostPanel.getByTestId('party-current')).toContainText(title(FIRST));
	await expect(hostPanel.getByTestId('party-tab-members')).toHaveText(/Members\s*2/);
	await hostPanel.getByTestId('party-avatars').click();
	const people = hostPanel.getByTestId('party-members');
	await expect(people.getByText('Grace')).toBeVisible();
	await expect(people.getByRole('button', { name: 'Remove Grace' })).toBeVisible();
	await expect(await partyMenu(host, 'End party')).toBeVisible();
	await host.keyboard.press('Escape');
	await hostPanel.getByTestId('party-tab-queue').click();
	const guestPanel = await openPanel(guest);
	await guestPanel.getByTestId('party-tab-members').click();
	await expect(
		guestPanel.getByTestId('party-members').getByRole('button', { name: /^Remove/ })
	).toHaveCount(0);
	await expect(await partyMenu(guest, 'Leave party')).toBeVisible();
	await expect(guest.getByRole('menuitem', { name: 'End party' })).toHaveCount(0);
	await guest.keyboard.press('Escape');
	await closePanel(guest);

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

	// The guest browses on: the party video moves to the mini player, and another one is queued.
	await browseTo(guest, SECOND);
	await expect(guest.getByTestId('party-elsewhere')).toContainText(title(FIRST));
	await expect(guest.getByTestId('mini-player-title')).toHaveText(title(FIRST));
	await guest.getByTestId('add-to-party').click();
	await expect(guest.getByTestId('add-to-party')).toHaveText('Added to party');
	const queue = hostPanel.getByTestId('party-queue');
	await expect(queue.getByText(title(SECOND))).toBeVisible();
	await expect(queue.getByText('Added by Grace')).toBeVisible();

	// When the first video ends, the host on its watch page follows to the next one; the guest
	// already is on that page, where the party video now plays.
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect(host).toHaveURL(watchUrl(SECOND), { timeout: 15_000 });
	await expect(host.getByRole('heading', { name: title(SECOND) })).toBeVisible();
	await expect(guest.getByTestId('party-elsewhere')).toBeHidden();
	await expect(guest.getByTestId('player-slot')).toBeVisible();
	await expect(guest.getByTestId('app-player')).toBeVisible();
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

/** Each player may sit up to 30 ms off the party before it corrects, in opposite directions. */
const SETTLED_GAP = 0.05;

async function joinParty(browser: Browser, room: FakeRoom, query = '') {
	const hostContext = await browser.newContext();
	const guestContext = await browser.newContext();
	await mockApi(hostContext, room, { signedIn: true });
	await mockApi(guestContext, room, { signedIn: false });
	const host = await hostContext.newPage();
	const guest = await guestContext.newPage();
	const first = room.state().current!.video.id;
	await openParty(host, query);
	await expect(host).toHaveURL(watchUrl(first));
	await joinAsGuest(guest, query);
	await expect(guest).toHaveURL(watchUrl(first));
	await expect(host.getByTestId('party-count')).toHaveText('2');
	return { host, guest, hostContext, guestContext };
}

test('players stay within a few frames of each other', async ({ browser }) => {
	test.setTimeout(90_000);
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room, '?debug=sync');

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
		.toBeLessThan(SETTLED_GAP);
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
		.toBeLessThan(SETTLED_GAP);
	const afterSeek = await worstGap(host, guest, 3);
	console.log(`gap after a seek: ${(afterSeek * 1000).toFixed(1)} ms`);
	expect(afterSeek).toBeLessThan(SETTLED_GAP);

	// Paused, both show the same frame.
	await media(guest).evaluate((el: HTMLMediaElement) => el.pause());
	await expect.poll(() => isPaused(host)).toBe(true);
	await expect.poll(async () => Math.abs(await gap(host, guest))).toBeLessThan(0.04);

	// No watch progress is recorded in a party.
	expect(await guest.evaluate(() => localStorage.getItem('oxistream:progress'))).toBeNull();

	await hostContext.close();
	await guestContext.close();
});

test('the party plays on while browsing and leaving asks first', async ({ browser }) => {
	test.setTimeout(60_000);
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);
	await expect.poll(() => hasMetadata(host)).toBe(true);
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect.poll(() => isPaused(guest)).toBe(false);

	// Off the watch page, the party video goes on in the mini player.
	await guest.getByRole('link', { name: 'Home' }).first().click();
	await expect(guest).toHaveURL(/\/$/);
	const mini = guest.getByTestId('mini-player');
	await expect(mini).toBeVisible();
	await expect(guest.getByTestId('mini-player-title')).toHaveText(title(LONG));
	await expect.poll(() => isPaused(guest)).toBe(false);
	expect(room.playing).toBe(true);

	// Another member switching the video changes the mini player, without moving the guest.
	await browseTo(host, LONG_TOO);
	await host.getByTestId('add-to-party').click();
	const queue = (await openPanel(host)).getByTestId('party-queue');
	await queue.getByRole('button', { name: new RegExp(`^${title(LONG_TOO)}`) }).click();
	await expect(guest.getByTestId('mini-player-title')).toHaveText(title(LONG_TOO));
	await expect(guest).toHaveURL(/\/$/);
	await expect(host).toHaveURL(watchUrl(LONG_TOO));
	await expect(host.getByTestId('player-slot')).toBeVisible();
	await expect.poll(() => isPaused(guest)).toBe(false);

	// Closing the mini player leaves the party, after asking.
	await mini.getByRole('button', { name: 'Leave party' }).click();
	const dialog = guest.getByRole('alertdialog');
	await expect(dialog).toContainText('Leave the party?');
	await dialog.getByRole('button', { name: 'Cancel' }).click();
	await expect(mini).toBeVisible();
	await mini.getByRole('button', { name: 'Leave party' }).click();
	await guest.getByTestId('party-leave-confirm').click();
	await expect(mini).toBeHidden();
	await expect(guest.getByTestId('party-button')).toHaveCount(0);
	await expect(host.getByTestId('party-count')).toHaveText('1');
	expect(room.playing).toBe(true);

	await hostContext.close();
	await guestContext.close();
});

const currentTime = (page: Page) => media(page).evaluate((el: HTMLMediaElement) => el.currentTime);

/** Drags the time slider from where playback is to `fraction` of its width. */
async function scrub(page: Page, fraction: number) {
	const player = page.getByTestId('app-player');
	await player.hover();
	const slider = player.locator('media-time-slider');
	await expect(slider).toBeVisible();
	const box = (await slider.boundingBox())!;
	const y = box.y + box.height / 2;
	const from = box.x + box.width * ((await currentTime(page)) / 60);
	const to = box.x + box.width * fraction;
	await page.mouse.move(from, y);
	await page.mouse.down();
	for (let i = 1; i <= 5; i++) {
		await page.mouse.move(from + ((to - from) * i) / 5, y);
		await page.waitForTimeout(100);
	}
	await page.mouse.up();
}

/** Where the party is now. */
const live = (room: FakeRoom) =>
	room.position + (room.playing ? (Date.now() - room.positionAt) / 1000 : 0);

/** The player stays with the party instead of snapping back to where it was before. */
async function staysWithRoom(page: Page, room: FakeRoom, seconds: number) {
	for (let i = 0; i < seconds * 4; i++) {
		expect(Math.abs((await currentTime(page)) - live(room))).toBeLessThan(3);
		await page.waitForTimeout(250);
	}
}

test('scrubbing in a party moves everyone there', async ({ browser }) => {
	test.setTimeout(90_000);
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);

	await expect.poll(() => hasMetadata(host)).toBe(true);
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect.poll(() => isPaused(guest)).toBe(false);
	await expect.poll(() => currentTime(guest)).toBeGreaterThan(1);
	const follows = async () => Math.abs((await currentTime(host)) - live(room));

	// While playing, with the time slider.
	await scrub(guest, 0.7);
	await expect.poll(() => room.position).toBeGreaterThan(38);
	expect(room.position).toBeLessThan(46);
	await staysWithRoom(guest, room, 2);
	await expect.poll(follows).toBeLessThan(0.5);

	// With the keyboard, a second per step.
	const before = await currentTime(guest);
	const dragged = room.position;
	await guest.getByTestId('app-player').locator('media-time-slider').focus();
	// Paced like a person: the slider steps from the time it last showed.
	for (let i = 0; i < 5; i++) {
		await guest.keyboard.press('ArrowLeft');
		await guest.waitForTimeout(150);
	}
	// The party was already a little behind `before` after the drag; wait for the steps.
	await expect.poll(() => room.position !== dragged && room.position < before - 2).toBe(true);
	await expect
		.poll(async () => Math.abs((await currentTime(guest)) - live(room)))
		.toBeLessThan(0.5);
	await staysWithRoom(guest, room, 2);
	await expect.poll(follows).toBeLessThan(0.5);

	// While paused.
	await media(guest).evaluate((el: HTMLMediaElement) => el.pause());
	await expect.poll(() => isPaused(host)).toBe(true);
	await scrub(guest, 0.2);
	await expect.poll(() => room.position).toBeLessThan(16);
	expect(room.position).toBeGreaterThan(8);
	await staysWithRoom(guest, room, 2);
	await expect.poll(follows).toBeLessThan(0.1);
	expect(room.playing).toBe(false);

	await hostContext.close();
	await guestContext.close();
});

test('another video can play now for everyone or join the queue', async ({ browser }) => {
	test.setTimeout(60_000);
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);
	await expect.poll(() => hasMetadata(host)).toBe(true);
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect.poll(() => isPaused(guest)).toBe(false);

	// Another watch page offers both; the party video plays on in the mini player meanwhile.
	await browseTo(host, LONG_TOO);
	const elsewhere = host.getByTestId('party-elsewhere');
	await expect(elsewhere).toContainText(title(LONG));
	await expect(host.getByTestId('mini-player-title')).toHaveText(title(LONG));
	await expect.poll(() => isPaused(host)).toBe(false);
	await elsewhere.getByRole('button', { name: 'Add to queue' }).click();
	await expect(host.getByTestId('party-queue-add')).toHaveText('Queued');
	await expect.poll(() => room.state().queue.map((e) => e.video.id)).toEqual([LONG_TOO]);
	await expect(guest).toHaveURL(watchUrl(LONG));
	expect(room.state().current?.video.id).toBe(LONG);

	// Back on its page later, it is still in the queue; "Play now" takes it from there.
	await browseTo(host, SECOND);
	await browseTo(host, LONG_TOO);
	await expect(host.getByTestId('party-queue-add')).toHaveText('Already in the queue');
	await host.getByTestId('party-play-now').click();
	await expect(host.getByTestId('player-slot')).toBeVisible();
	await expect(host.getByTestId('party-elsewhere')).toBeHidden();
	// The guest was on the previous video's page, so follows.
	await expect(guest).toHaveURL(watchUrl(LONG_TOO));
	await expect(guest.getByTestId('mini-player-title')).toHaveCount(0);
	expect(room.state().queue).toEqual([]);

	// A video that is not queued yet plays right away, too.
	await browseTo(guest, LONG);
	await guest.getByTestId('party-play-now').click();
	await expect(guest.getByTestId('player-slot')).toBeVisible();
	await expect(host).toHaveURL(watchUrl(LONG));
	await expect.poll(() => isPaused(host)).toBe(false);
	expect(room.state().queue).toEqual([]);
	expect(room.state().current?.added_by).toBe('Grace');

	await hostContext.close();
	await guestContext.close();
});

test('outside a party, another video plays right away', async ({ browser }) => {
	const context = await browser.newContext();
	await mockApi(context, new FakeRoom(), { signedIn: false });
	const page = await context.newPage();
	await page.goto(`/watch?v=${FIRST}`);
	await expect(page.getByTestId('player-slot')).toBeVisible();
	const manifest = page.waitForRequest(`**/api/v0/videos/${SECOND}/manifest.mpd`);
	await browseTo(page, SECOND);
	await manifest;
	await expect(page.getByTestId('player-slot')).toBeVisible();
	await expect(page.getByTestId('party-elsewhere')).toHaveCount(0);
	await expect(page.getByTestId('party-play-now')).toHaveCount(0);
	await context.close();
});

test('shorts mute the party video, and a video that cannot play says so', async ({ browser }) => {
	test.setTimeout(60_000);
	const room = new FakeRoom(LONG);
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);
	await expect.poll(() => hasMetadata(host)).toBe(true);
	await media(host).evaluate((el: HTMLMediaElement) => el.play());
	await expect.poll(() => isPaused(guest)).toBe(false);
	const muted = (page: Page) => media(page).evaluate((el: HTMLMediaElement) => el.muted);

	// On shorts, the party video goes on muted in the mini player.
	await guest.getByRole('link', { name: 'Shorts' }).first().click();
	await expect(guest).toHaveURL(/\/shorts/);
	await expect(guest.getByTestId('mini-player-muted')).toBeVisible();
	await expect.poll(() => muted(guest)).toBe(true);
	await expect.poll(() => isPaused(guest)).toBe(false);
	expect(await muted(host)).toBe(false);
	await guest.getByRole('link', { name: 'Home' }).first().click();
	await expect(guest.getByTestId('mini-player-muted')).toHaveCount(0);
	await expect.poll(() => muted(guest)).toBe(false);
	// Not saved as the guest's preference.
	const saved = await guest.evaluate(() => localStorage.getItem('oxistream:player'));
	expect((JSON.parse(saved ?? '{}') as { muted?: boolean }).muted ?? false).toBe(false);

	// A livestream cannot play: the player area and the mini player say so.
	room.add(LIVE, 'mara', true);
	room.broadcast();
	await expect(host).toHaveURL(watchUrl(LIVE));
	const message = host.getByTestId('app-player').getByTestId('player-message');
	await expect(message).toHaveAttribute('data-kind', 'live');
	await expect(message).toContainText('Livestreams are not supported yet');
	await expect(guest.getByTestId('mini-player').getByTestId('player-message')).toBeVisible();
	await expect(guest.getByTestId('mini-player-title')).toHaveText(title(LIVE));
	await expect(guest.getByTestId('mini-player-toggle')).toHaveCount(0);

	await hostContext.close();
	await guestContext.close();
});

type Box = { x: number; y: number; width: number; height: number };
const box = async (locator: Locator): Promise<Box> => (await locator.boundingBox())!;
const right = (b: Box) => b.x + b.width;

test('the party sidebar sits next to the content and stays open across visits', async ({
	browser
}) => {
	const context = await browser.newContext();
	await mockApi(context, new FakeRoom(LONG), { signedIn: true });
	const page = await context.newPage();
	const button = page.getByTestId('party-button');
	const panel = page.getByTestId('party-panel');
	const card = page.locator('[data-slot="sidebar-inset"]');
	const viewport = page.viewportSize()!;

	// Joining opens it as a sidebar, and the body card makes room instead of being covered.
	await openParty(page);
	await expect(page).toHaveURL(watchUrl(LONG));
	await expect(page.getByTestId('party-sidebar')).toBeVisible();
	await expect(panel).toBeVisible();
	await expect(button).toHaveAttribute('aria-expanded', 'true');
	await expect(page.getByRole('dialog')).toHaveCount(0);
	await expect.poll(async () => right(await box(card))).toBeLessThan(viewport.width - 300);
	const aside = page.locator('#party-panel');
	// Both move during the width transition, so they are measured together.
	await expect
		.poll(async () => (await box(aside)).x - right(await box(card)))
		.toBeGreaterThanOrEqual(-1);
	await expect.poll(async () => right(await box(aside))).toBeLessThanOrEqual(viewport.width + 1);

	// The mini player stays inside the body card.
	await page.getByRole('link', { name: 'Home' }).first().click();
	const mini = page.getByTestId('mini-player');
	await expect(mini).toBeVisible();
	await expect.poll(async () => right(await box(mini))).toBeLessThanOrEqual(right(await box(card)));
	expect(right(await box(mini))).toBeLessThan((await box(aside)).x);

	// Closed, the body card takes the full width again, also after a reload.
	await button.click();
	await expect(panel).toBeHidden();
	await expect(button).toHaveAttribute('aria-expanded', 'false');
	await expect.poll(async () => right(await box(card))).toBeGreaterThan(viewport.width - 16);
	await expect.poll(async () => right(await box(mini))).toBeGreaterThan(viewport.width - 32);
	await page.reload();
	await expect(button).toHaveAttribute('aria-expanded', 'false');
	await expect(panel).toBeHidden();

	await button.click();
	await expect(panel).toBeVisible();
	await page.reload();
	await expect(button).toHaveAttribute('aria-expanded', 'true');
	await expect(panel).toBeVisible();
	await expect(panel.getByTestId('party-invite')).toHaveAttribute(
		'data-url',
		new RegExp(`/party/${ROOM}$`)
	);

	await context.close();
});

test('on a phone the party opens as a sheet', async ({ browser }) => {
	const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
	await mockApi(context, new FakeRoom(LONG), { signedIn: true });
	const page = await context.newPage();
	await openParty(page);
	await expect(page).toHaveURL(watchUrl(LONG));

	const sheet = page.getByRole('dialog');
	await expect(sheet).toBeVisible();
	await expect(sheet.getByTestId('party-panel')).toBeVisible();
	await expect(page.getByTestId('party-sidebar')).toHaveCount(0);
	await sheet.getByRole('button', { name: 'Close' }).click();
	await expect(sheet).toBeHidden();

	await page.getByTestId('party-button').click();
	await expect(sheet).toBeVisible();
	await expect(sheet.getByTestId('party-invite')).toHaveAttribute(
		'data-url',
		new RegExp(`/party/${ROOM}$`)
	);
	await page.keyboard.press('Escape');
	await expect(sheet).toBeHidden();

	// The sheet is not remembered.
	await page.reload();
	await expect(page.getByTestId('party-button')).toHaveAttribute('aria-expanded', 'false');
	await expect(sheet).toHaveCount(0);

	await context.close();
});

test('the invite menu copies the link and shows a scannable QR code', async ({
	browser,
	browserName
}) => {
	const context = await browser.newContext({
		permissions: browserName === 'chromium' ? ['clipboard-read', 'clipboard-write'] : []
	});
	await mockApi(context, new FakeRoom(LONG), { signedIn: true });
	const page = await context.newPage();
	await page.addInitScript(() => delete (Navigator.prototype as Partial<Navigator>).share);
	await openParty(page);
	const invite = page.getByTestId('party-panel').getByTestId('party-invite');
	await expect(invite).toHaveAttribute('data-url', new RegExp(`/party/${ROOM}$`));
	const url = await invite.getAttribute('data-url');

	await invite.click();
	await page.getByTestId('party-invite-copy').click();
	await expect(invite).toHaveAttribute('data-copied', 'true');
	expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(url);

	await invite.click();
	await page.getByTestId('party-invite-qr').click();
	const qr = page.getByRole('dialog').getByTestId('party-qr');
	await expect(qr).toBeVisible();
	expect((await qr.boundingBox())!.width).toBeGreaterThanOrEqual(200);
	const image = await qr.evaluate(async (svg) => {
		const size = 400;
		const img = new Image();
		img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
		await img.decode();
		const canvas = document.createElement('canvas');
		canvas.width = canvas.height = size;
		const ctx = canvas.getContext('2d')!;
		ctx.drawImage(img, 0, 0, size, size);
		return { size, data: Array.from(ctx.getImageData(0, 0, size, size).data) };
	});
	expect(jsQR(Uint8ClampedArray.from(image.data), image.size, image.size)?.data).toBe(url);

	await page.keyboard.press('Escape');
	await expect(qr).toBeHidden();

	// Without the Web Share API there is no share item.
	await invite.click();
	await expect(page.getByTestId('party-invite-copy')).toBeVisible();
	await expect(page.getByTestId('party-invite-share')).toHaveCount(0);
	await page.keyboard.press('Escape');

	await page.addInitScript(() => {
		navigator.share = (data) => {
			(window as unknown as { shared: ShareData }).shared = data!;
			return Promise.resolve();
		};
	});
	await page.reload();
	await invite.click();
	await page.getByTestId('party-invite-share').click();
	await expect
		.poll(() => page.evaluate(() => (window as unknown as { shared?: ShareData }).shared?.url))
		.toBe(url);
	await context.close();
});

test('the queue is filled from the panel, reordered and shuffled', async ({ browser }) => {
	test.setTimeout(60_000);
	const room = new FakeRoom();
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);
	const PLAYLIST = 'evening-list-001';
	const playlist = {
		id: PLAYLIST,
		title: 'Evening',
		description: '',
		visibility: 'private',
		item_count: 3,
		thumbnails: [],
		created_at: '2026-11-26T10:00:00Z',
		updated_at: '2026-11-26T10:00:00Z'
	};
	await hostContext.route('**/api/v0/search?*', (route) =>
		json(route, {
			items: [LIVE, SECOND, LONG].map((id) => ({ ...summary(id), type: 'video' })),
			continuation: null
		})
	);
	await hostContext.route('**/api/v0/resolve?*', (route) =>
		json(route, { kind: 'video', id: LONG_TOO })
	);
	await hostContext.route('**/api/v0/playlists', (route) => json(route, [playlist]));
	await hostContext.route(`**/api/v0/playlists/${PLAYLIST}`, (route) =>
		json(route, {
			...playlist,
			owner: 'mara',
			is_owner: true,
			items: [FIRST, SECOND, LIVE].map((id) => ({
				video: summary(id),
				added_at: '2026-11-26T10:00:00Z'
			}))
		})
	);
	const ids = () => room.state().queue.map((e) => e.video.id);

	// Search results queue with one click and say so; livestreams are left out.
	const hostPanel = await openPanel(host);
	await hostPanel.getByTestId('party-add-input').fill('fjord');
	const results = hostPanel.getByTestId('party-add-results').getByTestId('party-pick');
	await expect(results).toHaveCount(2);
	await results.first().click();
	await expect(results.first()).toHaveAttribute('data-queued', 'true');
	await expect(results.first()).toBeDisabled();
	await expect.poll(ids).toEqual([SECOND]);

	// A pasted link is queued right away.
	await hostPanel.getByTestId('party-add-input').fill('');
	await hostPanel.getByTestId('party-add-input').evaluate((input) => {
		const data = new DataTransfer();
		data.setData('text/plain', 'https://youtu.be/syncDDDDDDw');
		input.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true }));
	});
	await expect.poll(ids).toEqual([SECOND, LONG_TOO]);
	await expect(hostPanel.getByTestId('party-add-input')).toHaveValue('');

	// A whole playlist from the library; what cannot be queued is counted.
	await hostPanel.getByTestId('party-library-open').click();
	const library = host.getByTestId('party-library');
	await library.getByRole('button', { name: /Evening/ }).click();
	await expect(library.getByTestId('party-pick')).toHaveCount(3);
	await library.getByTestId('party-library-all').click();
	await expect(hostPanel.getByTestId('party-batch')).toHaveText('2 queued, 1 skipped');
	await expect.poll(ids).toEqual([SECOND, LONG_TOO, FIRST, SECOND]);

	// Guests have no library, but reorder with the keyboard like everyone.
	const guestPanel = await openPanel(guest);
	await expect(guestPanel.getByTestId('party-library-open')).toHaveCount(0);
	const guestQueue = guestPanel.getByTestId('party-queue');
	await guestQueue.getByTestId('party-queue-handle').first().focus();
	await guest.keyboard.press('End');
	await expect.poll(ids).toEqual([LONG_TOO, FIRST, SECOND, SECOND]);
	await expect(guestQueue.getByTestId('party-queue-handle').last()).toBeFocused();

	// Dragging the last entry to the top.
	const handles = hostPanel.getByTestId('party-queue-handle');
	await expect(handles).toHaveCount(4);
	await handles.last().scrollIntoViewIfNeeded();
	const from = (await handles.last().boundingBox())!;
	const to = (await handles.first().boundingBox())!;
	await host.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
	await host.mouse.down();
	await host.mouse.move(to.x + to.width / 2, to.y + 4, { steps: 8 });
	await host.mouse.up();
	await expect.poll(ids).toEqual([SECOND, LONG_TOO, FIRST, SECOND]);
	const titles = hostPanel.getByTestId('party-queue-item');
	await expect(titles.first()).toContainText(title(SECOND));
	await expect(titles.nth(1)).toContainText(title(LONG_TOO));

	// A move based on an older state is dropped without a notice.
	room.revision++;
	await guestQueue.getByTestId('party-queue-handle').first().focus();
	await guest.keyboard.press('ArrowDown');
	await guest.waitForTimeout(300);
	expect(ids()).toEqual([SECOND, LONG_TOO, FIRST, SECOND]);
	await expect(guestPanel.getByRole('alert')).toHaveCount(0);

	await guestPanel.getByTestId('party-shuffle').click();
	await expect.poll(ids).toEqual([SECOND, FIRST, LONG_TOO, SECOND]);
	await expect(titles.nth(2)).toContainText(title(LONG_TOO));

	await hostContext.close();
	await guestContext.close();
});

test('the now playing card shows progress, and autoplay continues with a related video', async ({
	browser
}) => {
	test.setTimeout(60_000);
	const room = new FakeRoom();
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);

	// Everyone sees where the party is.
	const hostPanel = await openPanel(host);
	const guestPanel = await openPanel(guest);
	await expect(guestPanel.getByTestId('party-time')).toHaveText('0:00 / 0:06');
	await expect(guestPanel.getByTestId('party-progress')).toHaveAttribute('aria-valuenow', '0');

	// Anyone may turn on autoplay; everyone sees the same preview instead of an empty queue.
	const toggle = guestPanel.getByTestId('party-autoplay');
	await expect(toggle).toHaveAttribute('aria-checked', 'false');
	await toggle.click();
	await expect.poll(() => room.autoplay).toBe(true);
	await expect(hostPanel.getByTestId('party-autoplay')).toHaveAttribute('aria-checked', 'true');
	const upNext = hostPanel.getByTestId('party-up-next');
	await expect(upNext).toContainText(title(SECOND));
	await expect(hostPanel.getByTestId('party-queue')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-up-next')).toContainText(title(SECOND));

	// While playing, the time runs for those who only watch the panel, too. The autoplay
	// state can still pause a play() that started right after the toggle, so retry it.
	await expect
		.poll(async () => {
			await media(host).evaluate((el: HTMLMediaElement) => el.play().catch(() => {}));
			return room.playing;
		})
		.toBe(true);
	await expect(guestPanel.getByTestId('party-time')).toHaveText(/^0:0[1-6] \/ 0:06$/);

	// When the last entry ends, the preview plays and the next one is picked.
	await expect(hostPanel.getByTestId('party-current')).toContainText(title(SECOND), {
		timeout: 15_000
	});
	expect(room.state().current?.autoplay).toBe(true);
	await expect(upNext).toContainText(title(LONG));
	await expect.poll(() => isPaused(guest)).toBe(false);

	// Turned off, the preview goes away.
	await hostPanel.getByTestId('party-autoplay').click();
	await expect(guestPanel.getByTestId('party-up-next')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-queue')).toContainText('Nothing up next.');

	await hostContext.close();
	await guestContext.close();
});

test('played videos stay in the history, to play or queue again', async ({ browser }) => {
	test.setTimeout(60_000);
	const room = new FakeRoom();
	room.add(SECOND, 'Grace');
	room.add(LONG, 'Grace');
	const { host, guest, hostContext, guestContext } = await joinParty(browser, room);
	const ids = () => room.state().queue.map((e) => e.video.id);

	const guestPanel = await openPanel(guest);
	await guestPanel.getByTestId('party-tab-history').click();
	await expect(guestPanel.getByTestId('party-history')).toContainText('Nothing played yet.');

	// Playing another entry moves the current one to the history, for everyone.
	await guestPanel.getByTestId('party-tab-queue').click();
	await guestPanel
		.getByTestId('party-queue-item')
		.filter({ hasText: title(SECOND) })
		.click();
	await expect(guestPanel.getByTestId('party-current')).toContainText(title(SECOND));
	await guestPanel.getByTestId('party-tab-history').click();
	const guestHistory = guestPanel.getByTestId('party-history-item');
	await expect(guestHistory).toHaveCount(1);
	await expect(guestHistory.first()).toContainText(title(FIRST));
	await expect(guestHistory.first()).toContainText('Added by mara');

	// The menu queues it again, once.
	const hostPanel = await openPanel(host);
	await hostPanel.getByTestId('party-tab-history').click();
	await hostPanel.getByTestId('party-history-menu').first().click();
	await host.getByTestId('party-history-queue').click();
	await expect.poll(ids).toEqual([LONG, FIRST]);
	await hostPanel.getByTestId('party-history-menu').first().click();
	await expect(host.getByTestId('party-history-queue')).toHaveAttribute('data-disabled', '');
	await expect(host.getByTestId('party-history-queue')).toHaveText('Already in the queue');
	await host.keyboard.press('Escape');

	// Clicking one plays it again for everyone; the newest comes first.
	await guestHistory.first().click();
	await expect(hostPanel.getByTestId('party-current')).toContainText(title(FIRST));
	await expect.poll(ids).toEqual([LONG]);
	const hostHistory = hostPanel.getByTestId('party-history-item');
	await expect(hostHistory).toHaveCount(2);
	await expect(hostHistory.first()).toContainText(title(SECOND));
	await expect(hostHistory.first()).toContainText('Added by Grace');
	await expect(hostHistory.nth(1)).toContainText(title(FIRST));

	await hostContext.close();
	await guestContext.close();
});

test('members show how they keep up and where they are, and who comes and goes', async ({
	browser
}) => {
	test.setTimeout(60_000);
	const room = new FakeRoom(LONG);
	const hostContext = await browser.newContext();
	const guestContext = await browser.newContext();
	await mockApi(hostContext, room, { signedIn: true });
	await mockApi(guestContext, room, { signedIn: false });
	const host = await hostContext.newPage();
	const guest = await guestContext.newPage();

	await openParty(host);
	await expect(host).toHaveURL(watchUrl(LONG));
	const hostPanel = await openPanel(host);
	await hostPanel.getByTestId('party-tab-members').click();
	const member = (panel: Locator, name: string) =>
		panel.locator(`[data-testid="party-member"][data-name="${name}"]`);
	await expect(member(hostPanel, 'mara')).toHaveAttribute('data-place', 'watching');
	await expect(hostPanel.getByTestId('party-presence')).toBeHidden();

	// Someone joining is announced; their own join is not.
	await joinAsGuest(guest);
	await expect(guest).toHaveURL(watchUrl(LONG));
	await expect(hostPanel.getByTestId('party-presence')).toHaveText('Grace joined', {
		timeout: 10_000
	});
	const guestPanel = await openPanel(guest);
	await expect(guestPanel.getByTestId('party-presence')).toBeHidden();
	const grace = member(hostPanel, 'Grace');
	await expect(grace).toHaveAttribute('data-place', 'watching');
	await expect(grace).toHaveAttribute('data-sync', 'synced', { timeout: 15_000 });
	await expect(grace).toContainText('Watching');

	// Only your own row can be resynced.
	await expect(grace.getByTestId('party-resync')).toHaveCount(0);
	await expect(member(hostPanel, 'mara').getByTestId('party-resync')).toBeVisible();
	await guestPanel.getByTestId('party-tab-members').click();
	await expect(member(guestPanel, 'mara').getByTestId('party-resync')).toHaveCount(0);
	await expect.poll(() => hasMetadata(host)).toBe(true);
	await expect.poll(() => hasMetadata(guest)).toBe(true);
	await expect
		.poll(async () => {
			await media(host).evaluate((el: HTMLMediaElement) => void el.play().catch(() => {}));
			return room.playing;
		})
		.toBe(true);
	await expect.poll(() => isPaused(guest)).toBe(false);
	await member(guestPanel, 'Grace').getByTestId('party-resync').click();
	await expect
		.poll(async () => Math.abs((await currentTime(guest)) - live(room)), { timeout: 10_000 })
		.toBeLessThan(1);
	await expect(grace).toHaveAttribute('data-sync', 'synced', { timeout: 15_000 });

	// Browsing elsewhere shows for everyone.
	await browseTo(guest, SECOND);
	await expect(grace).toHaveAttribute('data-place', 'browsing');
	await expect(grace).toContainText('Browsing');

	// Leaving is announced once the reconnect grace is over.
	await guestContext.close();
	await expect(grace).toHaveCount(0);
	await expect(hostPanel.getByTestId('party-presence')).toHaveText('Grace left', {
		timeout: 15_000
	});

	await hostContext.close();
});

test('members chat and react, and the host decides who controls the party', async ({ browser }) => {
	test.setTimeout(60_000);
	const room = new FakeRoom(LONG);
	const hostContext = await browser.newContext();
	const guestContext = await browser.newContext();
	await mockApi(hostContext, room, { signedIn: true });
	await mockApi(guestContext, room, { signedIn: false });
	const host = await hostContext.newPage();
	const guest = await guestContext.newPage();

	await openParty(host);
	await expect(host).toHaveURL(watchUrl(LONG));
	await joinAsGuest(guest);
	await expect(guest).toHaveURL(watchUrl(LONG));
	const hostPanel = await openPanel(host);
	await hostPanel.getByTestId('party-tab-chat').click();
	await expect(hostPanel.getByTestId('party-chat')).toContainText('No messages yet.');

	// Unread messages mark the chat tab, and the party button while the panel is closed.
	const input = hostPanel.getByTestId('party-chat-input');
	await input.fill('  Hello   there  ');
	await input.press('Enter');
	await expect(input).toHaveValue('');
	const hostMessages = hostPanel.getByTestId('party-chat-message');
	await expect(hostMessages).toHaveText([/Hello there/]);
	await expect(hostMessages.first()).toHaveAttribute('data-own', '');
	await expect(hostPanel.getByTestId('party-chat-unread')).toHaveCount(0);
	let guestPanel = await openPanel(guest);
	await expect(guestPanel.getByTestId('party-chat-unread')).toContainText('1');
	await expect(guest.getByTestId('party-unread')).toHaveCount(0);
	await closePanel(guest);
	await expect(guest.getByTestId('party-unread')).toBeVisible();
	guestPanel = await openPanel(guest);
	await guestPanel.getByTestId('party-tab-chat').click();
	await expect(guestPanel.getByTestId('party-chat-unread')).toHaveCount(0);
	const guestMessages = guestPanel.getByTestId('party-chat-message');
	await expect(guestMessages.first()).toContainText('mara');
	await expect(guestMessages.first()).toContainText('Host');
	await expect(guestMessages.first()).not.toHaveAttribute('data-own');

	// Messages are plain text.
	await guestPanel.getByTestId('party-chat-input').fill('<b>bold</b>');
	await guestPanel.getByTestId('party-chat-send').click();
	await expect(hostMessages.nth(1)).toContainText('<b>bold</b>');
	await expect(hostPanel.locator('[data-testid="party-chat"] b')).toHaveCount(0);

	// Reactions float over everyone's player.
	await guestPanel.getByTestId('party-react-heart').click();
	await expect(
		host.locator('.app-player [data-testid="party-reaction"][data-reaction="heart"]')
	).toHaveCount(1);
	await expect(
		guest.locator('.app-player [data-testid="party-reaction"][data-reaction="heart"]')
	).toHaveCount(1);
	await expect(host.locator('[data-testid="party-reaction"]')).toHaveCount(0, { timeout: 5000 });

	// Playback is the host's alone: the guest loses the controls and snaps back.
	await expect.poll(() => hasMetadata(host)).toBe(true);
	await expect.poll(() => hasMetadata(guest)).toBe(true);
	await expect
		.poll(async () => {
			await media(host).evaluate((el: HTMLMediaElement) => void el.play().catch(() => {}));
			return room.playing;
		})
		.toBe(true);
	await expect.poll(() => isPaused(guest)).toBe(false);
	room.add(FIRST, 'mara', false, false, 'host');
	room.add(SECOND, 'mara', false, false, 'host');
	room.broadcast();
	await guestPanel.getByTestId('party-tab-queue').click();
	await expect(guestPanel.getByTestId('party-shuffle')).toBeVisible();
	await expect(await partyMenu(guest, 'Leave party')).toBeVisible();
	await expect(guest.getByTestId('party-permission-control-host')).toHaveCount(0);
	await guest.keyboard.press('Escape');

	await hostPanel.getByTestId('party-menu').click();
	await host.getByTestId('party-permission-control-host').click();
	await expect(host.getByTestId('party-permission-control-host')).toHaveAttribute(
		'data-state',
		'checked'
	);
	await host.keyboard.press('Escape');
	await expect.poll(() => room.permissions.control).toBe('host');
	await expect(guestPanel.getByTestId('party-shuffle')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-autoplay')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-queue-handle')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-queue-remove')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-queue-item').getByRole('button')).toHaveCount(0);
	await hostPanel.getByTestId('party-tab-queue').click();
	await expect(hostPanel.getByTestId('party-shuffle')).toBeVisible();

	const sentByGuest = () => room.received.filter((m) => !m.owner).map((m) => m.type);
	await media(guest).evaluate((el: HTMLMediaElement) => el.pause());
	await expect.poll(() => isPaused(guest), { timeout: 10_000 }).toBe(false);
	await media(guest).evaluate((el: HTMLMediaElement) => (el.currentTime = 50));
	await expect
		.poll(async () => Math.abs((await currentTime(guest)) - live(room)), { timeout: 10_000 })
		.toBeLessThan(2);
	expect(room.playing).toBe(true);
	expect(sentByGuest()).not.toContain('pause');
	expect(sentByGuest()).not.toContain('seek');

	// Others still queue their own videos and may take them out again.
	await browseTo(guest, LONG_TOO);
	await expect(guest.getByTestId('party-play-now')).toHaveCount(0);
	await guest.getByTestId('party-queue-add').click();
	await expect.poll(() => room.state().queue.map((e) => e.video.id)).toContain(LONG_TOO);
	const own = guestPanel.getByTestId('party-queue-item').filter({ hasText: title(LONG_TOO) });
	await own.getByTestId('party-queue-remove').click();
	await expect.poll(() => room.state().queue.map((e) => e.video.id)).not.toContain(LONG_TOO);

	// Adding can be the host's alone too.
	await hostPanel.getByTestId('party-menu').click();
	await host.getByTestId('party-permission-add-host').click();
	await host.keyboard.press('Escape');
	await expect.poll(() => room.permissions.add).toBe('host');
	await expect(guest.getByTestId('party-queue-add')).toHaveCount(0);
	await expect(guestPanel.getByTestId('party-add')).toHaveCount(0);
	await expect(hostPanel.getByTestId('party-add')).toBeVisible();

	await hostContext.close();
	await guestContext.close();
});
