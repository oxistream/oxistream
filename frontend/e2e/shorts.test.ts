import { readFileSync } from 'node:fs';
import { expect, test, type Page, type Route } from '@playwright/test';

// Autoplay with sound needs a user gesture otherwise, which a page load in a test does not have.
test.use({
	locale: 'en-US',
	launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] }
});

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);
const CHANNEL = 'UCm0ck4nv1lF0rg3H4mm3r1';
const shortId = (n: number) => `sho${String(n).padStart(7, '0')}A`;

// A grey 6 s VP9 clip: ffmpeg -f lavfi -i color=c=0x3f3f46:size=90x160:rate=15:duration=6
// -c:v libvpx-vp9 -b:v 20k -g 15 -f dash -seg_duration 1 -single_file 1 -dash_segment_type webm
const CLIP = readFileSync(new URL('./fixtures/short.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/short.mpd', import.meta.url), 'utf8');
const mpd = (id: string) => MPD.replace('{id}', id);

function serveClip(route: Route) {
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
}

function json(route: Route, body: unknown, status = 200) {
	return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

function summary(n: number) {
	return {
		id: shortId(n),
		title: `Forge short ${n}`,
		thumbnails: [{ url: `/proxy/image/s${n}-tall`, width: 405, height: 720 }],
		duration_seconds: null,
		view_count: 1000 * n,
		viewers: null,
		published_at: null,
		published_text: null,
		is_live: false,
		is_short: true,
		is_upcoming: false,
		channel: null,
		description: null
	};
}

function details(id: string, channelId: string) {
	const n = Number(id.slice(3, 10));
	return {
		id,
		title: `Forge short ${n}`,
		description: '',
		channel: {
			id: channelId,
			name: channelId ? 'Hammer & Tongs' : '',
			avatar: [],
			subscriber_count: null,
			verified: false
		},
		view_count: 1000,
		like_count: null,
		published_at: null,
		published_text: null,
		duration_seconds: 30,
		is_live: false,
		keywords: [],
		thumbnails: [],
		subtitles: [],
		related: []
	};
}

/** Three shorts on one channel; nothing reaches the backend or YouTube. */
async function mockApi(page: Page, { channelId = CHANNEL } = {}) {
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
	await page.route('**/proxy/stream/**', serveClip);
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/dash+xml',
			body: mpd(new URL(route.request().url()).pathname.split('/').at(-2)!)
		})
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) =>
		json(route, details(new URL(route.request().url()).pathname.split('/').pop()!, channelId))
	);
	await page.route(`**/api/v0/channels/${CHANNEL}`, (route) =>
		json(route, {
			id: CHANNEL,
			name: 'Hammer & Tongs',
			handle: '@hammerandtongs',
			avatar: [],
			verified: false,
			subscriber_count: 284000,
			video_count: 120,
			description: '',
			banner: [],
			tags: [],
			has_shorts: true,
			has_live: false,
			videos: { items: [], continuation: null }
		})
	);
	await page.route(`**/api/v0/channels/${CHANNEL}/shorts?*`, (route) =>
		json(route, { items: [1, 2, 3].map(summary), continuation: null })
	);
}

const viewer = (page: Page) => page.getByTestId('shorts-viewer');

/** How far a short's frame sits below the top of the stage, in px. */
function offsetFromStage(page: Page, id: string) {
	return page.evaluate((id) => {
		const frame = document.querySelector(`[data-short-id="${id}"]`);
		const stage = frame?.closest('.shorts-stage');
		if (!frame || !stage) return null;
		return Math.round(frame.getBoundingClientRect().top - stage.getBoundingClientRect().top);
	}, id);
}

test('arrow keys and j/k move between the channel shorts', async ({ page }) => {
	await mockApi(page);
	await page.goto('/');
	await page.goto(`/shorts/${shortId(2)}`);

	await expect(viewer(page).getByRole('heading', { name: 'Forge short 2' })).toBeVisible();
	await expect(page).toHaveTitle(/Forge short 2/);
	await expect(activeShort(page).getByTestId('short-watch-link')).toHaveAttribute(
		'href',
		`/watch?v=${shortId(2)}`
	);

	await expect(page.getByTestId('shorts-next')).toBeEnabled();
	await page.keyboard.press('ArrowDown');
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(3)}$`));
	await expect(viewer(page).getByRole('heading', { name: 'Forge short 3' })).toBeVisible();
	await expect.poll(() => offsetFromStage(page, shortId(3))).toBe(0);
	await expect(page.getByTestId('shorts-next')).toBeDisabled();

	await page.keyboard.press('k');
	await page.keyboard.press('ArrowUp');
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(1)}$`));
	await expect(page.getByTestId('shorts-previous')).toBeDisabled();

	await page.keyboard.press('j');
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(2)}$`));

	// The URL is replaced, so going back leaves the shorts.
	await page.goBack();
	await expect(page).toHaveURL(/\/$/);
});

const activeShort = (page: Page) => viewer(page).locator('[data-short-id][data-active]');
const playing = (page: Page) =>
	expect
		.poll(() =>
			activeShort(page)
				.locator('dash-video')
				.evaluate((media: HTMLMediaElement) => !media.paused && media.currentTime > 0)
		)
		.toBe(true);
const storedProgress = (page: Page) =>
	page.evaluate(() =>
		Object.keys(JSON.parse(localStorage.getItem('oxistream:progress') ?? '{}') as object)
	);

test('the next short is loaded ahead, and shorts keep no watch state', async ({ page }) => {
	await mockApi(page);
	const reports: string[] = [];
	page.on('request', (request) => {
		if (request.url().includes('/api/v0/progress')) reports.push(request.url());
	});
	const preloaded = page.waitForRequest(`**/proxy/stream/${shortId(2)}/**`);
	await page.goto(`/shorts/${shortId(1)}`);
	await preloaded;

	const next = viewer(page).locator(`[data-short-id="${shortId(2)}"]`);
	await expect(next).not.toHaveAttribute('data-active');
	await expect(next.locator('media-container')).toHaveAttribute('data-paused');

	await playing(page);
	await page.keyboard.press('Space');
	await expect(activeShort(page).locator('media-container')).toHaveAttribute('data-paused');
	await page.keyboard.press('Space');

	const third = page.waitForRequest(`**/proxy/stream/${shortId(3)}/**`);
	await page.keyboard.press('ArrowDown');
	await third;
	await expect(activeShort(page)).toHaveAttribute('data-short-id', shortId(2));
	await expect(activeShort(page).locator('media-container')).not.toHaveAttribute('data-paused');
	// The previous short keeps its player.
	await expect(viewer(page).locator(`[data-short-id="${shortId(1)}"] media-container`)).toHaveCount(
		1
	);

	// Neither paused nor left shorts end up in the history or "continue watching".
	await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pagehide')));
	expect(await storedProgress(page)).toEqual([]);
	expect(reports).toEqual([]);
});

test.describe('on a touch screen', () => {
	test.use({ hasTouch: true });

	test('a swipe drags the stack, then snaps to the next short or springs back', async ({
		page
	}) => {
		await mockApi(page);
		await page.goto(`/shorts/${shortId(1)}`);
		await expect(page.getByTestId('shorts-next')).toBeEnabled();
		const box = (await viewer(page).locator('.shorts-stage').boundingBox())!;
		const x = box.x + box.width / 2;
		const cdp = await page.context().newCDPSession(page);
		const touch = (type: 'touchStart' | 'touchMove' | 'touchEnd', y?: number) =>
			cdp.send('Input.dispatchTouchEvent', {
				type,
				touchPoints: y === undefined ? [] : [{ x, y }]
			});
		const startY = box.y + box.height * 0.7;

		// Slow and short: both shorts follow the finger, then the stack springs back.
		await touch('touchStart', startY);
		for (let dy = 10; dy <= 80; dy += 10) await touch('touchMove', startY - dy);
		await expect.poll(() => offsetFromStage(page, shortId(1))).toBe(-80);
		await expect.poll(() => offsetFromStage(page, shortId(2))).toBeLessThan(box.height);
		await page.waitForTimeout(200);
		await touch('touchEnd');
		await expect.poll(() => offsetFromStage(page, shortId(1))).toBe(0);
		await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(1)}$`));

		// Far enough: snaps to the next short.
		await touch('touchStart', startY);
		for (let dy = 40; dy <= box.height * 0.4; dy += 40) await touch('touchMove', startY - dy);
		await touch('touchEnd');
		await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(2)}$`));
		await expect.poll(() => offsetFromStage(page, shortId(2))).toBe(0);
		await expect(activeShort(page)).toHaveAttribute('data-short-id', shortId(2));
		await expect(activeShort(page).locator('media-container')).not.toHaveAttribute('data-paused');
	});
});

test('controls show only while paused; a tap or Space toggles playback', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/shorts/${shortId(1)}`);

	const frame = activeShort(page).locator('media-container');
	const controls = activeShort(page).locator('.video-controls-content');
	await playing(page);
	await expect(frame).not.toHaveAttribute('data-paused');
	await expect(controls).toBeHidden();

	await page.keyboard.press('Space');
	await expect(frame).toHaveAttribute('data-paused');
	await expect(controls).toBeVisible();

	await frame.click({ position: { x: 60, y: 120 } });
	await expect(frame).not.toHaveAttribute('data-paused');
	await expect(controls).toBeHidden();

	await frame.click({ position: { x: 60, y: 120 } });
	await expect(frame).toHaveAttribute('data-paused');
	await expect(activeShort(page).getByTestId('short-watch-link')).toBeVisible();
});

test('a short without a channel plays on its own', async ({ page }) => {
	await mockApi(page, { channelId: '' });
	await page.goto(`/shorts/${shortId(1)}`);

	await expect(viewer(page).getByRole('heading', { name: 'Forge short 1' })).toBeVisible();
	await expect(viewer(page).getByTestId('subscribe-button')).toHaveCount(0);
	await expect(page.getByTestId('shorts-next')).toBeDisabled();
	await page.keyboard.press('ArrowDown');
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(1)}$`));
});

const LINKED = 'fullVideo01';

function discovered(n: number) {
	return {
		id: shortId(n),
		channel_id: CHANNEL,
		thumbnails: [{ url: `/proxy/image/s${n}-tall`, width: 405, height: 720 }]
	};
}

/** Discovery in two pages; the first short links a full video. */
async function mockDiscovery(page: Page, { signedIn = false } = {}) {
	const requests: string[] = [];
	await page.route('**/api/v0/auth/me', (route) =>
		signedIn
			? json(route, {
					id: '0193a8e4-7f00-7000-8000-000000000001',
					username: 'mara',
					role: 'user',
					created_at: '2026-11-26T10:00:00Z'
				})
			: json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/shorts/discover*', (route) => {
		const continuation = new URL(route.request().url()).searchParams.get('continuation');
		requests.push(continuation ?? '');
		return continuation === 'p2'
			? json(route, { items: [3, 4].map(discovered), continuation: null })
			: json(route, { items: [1, 2].map(discovered), continuation: 'p2' });
	});
	await page.route(/\/api\/v0\/shorts\/sho[^/]+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop()!;
		return json(route, {
			id,
			title: null,
			linked_video:
				id === shortId(1) ? { id: LINKED, title: 'Forging the knife, start to end' } : null
		});
	});
	await page.route('**/api/v0/feed?*', (route) =>
		json(route, { items: [summary(7)], continuation: null })
	);
	return requests;
}

test('the shorts entry opens endless discovery with the linked full video', async ({ page }) => {
	await mockApi(page);
	const requests = await mockDiscovery(page);
	await page.goto('/shorts');

	const nav = page.getByRole('navigation', { name: 'Navigation' });
	await expect(nav.getByRole('link', { name: 'Shorts' })).toHaveAttribute('aria-current', 'page');
	await expect(viewer(page).getByRole('heading', { name: 'Forge short 1' })).toBeVisible();
	await expect(page.getByTestId('shorts-feeds')).toHaveCount(0);
	const chip = activeShort(page).getByTestId('short-linked-video');
	await expect(chip).toHaveText('Full video: Forging the knife, start to end');
	await expect(chip).toHaveAttribute('href', `/watch?v=${LINKED}`);

	// The second page loads as the end of the first comes near.
	await expect.poll(() => requests).toEqual(['', 'p2']);
	for (const n of [2, 3, 4]) {
		await page.keyboard.press('ArrowDown');
		await expect(viewer(page).getByRole('heading', { name: `Forge short ${n}` })).toBeVisible();
		await expect(activeShort(page).getByTestId('short-linked-video')).toHaveCount(0);
	}
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(4)}\\?feed=discover$`));
	await expect(page.getByTestId('shorts-next')).toBeDisabled();

	// Shorts shown in this session do not come again.
	await page.goto('/shorts');
	await expect(page.getByTestId('shorts-feed-state')).toContainText('No shorts right now');

	await page.goto(`/shorts/${shortId(1)}`);
	await activeShort(page).getByTestId('short-linked-video').click();
	await expect(page).toHaveURL(new RegExp(`/watch\\?v=${LINKED}$`));
});

test('signed-in users switch between discovery and subscriptions', async ({ page }) => {
	await mockApi(page);
	await mockDiscovery(page, { signedIn: true });
	await page.goto('/shorts');

	const feeds = page.getByTestId('shorts-feeds');
	await expect(feeds.getByRole('link', { name: 'For you' })).toHaveAttribute(
		'aria-current',
		'page'
	);
	await expect(viewer(page).getByRole('heading', { name: 'Forge short 1' })).toBeVisible();

	const feed = page.waitForRequest((r) => r.url().includes('/api/v0/feed?'));
	await feeds.getByRole('link', { name: 'Subscriptions' }).click();
	expect(new URL((await feed).url()).searchParams.get('kind')).toBe('shorts');
	await expect(feeds.getByRole('link', { name: 'Subscriptions' })).toHaveAttribute(
		'aria-current',
		'page'
	);
	await expect(viewer(page).getByRole('heading', { name: 'Forge short 7' })).toBeVisible();
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(7)}\\?feed=subscriptions$`));
});
