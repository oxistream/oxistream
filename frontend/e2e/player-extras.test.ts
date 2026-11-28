import { readFileSync } from 'node:fs';
import { expect, test, type Locator, type Page, type Route } from '@playwright/test';

test.use({
	locale: 'en-US',
	launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] }
});

const ID = 'extrasVid01';
const TITLE = 'Test pattern';
const CLIP = readFileSync(new URL('./fixtures/sync.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/sync.mpd', import.meta.url), 'utf8');

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

async function mockApi(page: Page) {
	// Nothing in these tests may reach the backend or YouTube.
	await page.route('**/proxy/**', (route) => route.fulfill({ status: 404, body: '' }));
	await page.route('**/proxy/stream/**', (route) => {
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
	await page.route('**/api/v0/**', (route) => json(route, { code: 'not_found', message: '' }, 404));
	await page.route('**/api/v0/auth/me', (route) =>
		json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/dash+xml',
			body: MPD.replace('{id}', ID)
		})
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) =>
		json(route, {
			id: ID,
			title: TITLE,
			description: '',
			thumbnails: [],
			channel: {
				id: 'UCnorth',
				name: 'Northbound',
				avatar: [],
				subscriber_count: null,
				verified: false
			},
			view_count: null,
			like_count: null,
			published_at: null,
			published_text: null,
			duration_seconds: 60,
			is_live: false,
			keywords: [],
			subtitles: [],
			related: []
		})
	);
}

type Media = HTMLMediaElement;
const time = (media: Locator) => media.evaluate((el) => (el as Media).currentTime);
const paused = (media: Locator) => media.evaluate((el) => (el as Media).paused);
const muted = (media: Locator) => media.evaluate((el) => (el as Media).muted);

/** Starts playback, retrying where the browser held back the first play. */
async function playing(page: Page, media: Locator) {
	await expect
		.poll(
			async () => {
				if (await paused(media)) await media.evaluate((el) => (el as Media).play().catch(() => {}));
				return time(media);
			},
			{ timeout: 15_000 }
		)
		.toBeGreaterThan(1);
}

async function openSettings(page: Page) {
	const player = page.getByTestId('app-player');
	await player.hover();
	await player.locator('.video-controls-settings-button').click();
}

test('YouTube shortcuts control the player and ? lists them', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await playing(page, media);

	// Focus on the page, not on the player.
	await page.getByRole('heading', { level: 1, name: TITLE }).click();

	await page.keyboard.press('k');
	await expect.poll(() => paused(media)).toBe(true);

	const start = await time(media);
	await page.keyboard.press('l');
	await expect.poll(() => time(media)).toBeGreaterThan(start + 9);
	const later = await time(media);
	await page.keyboard.press('ArrowLeft');
	await expect.poll(() => time(media)).toBeLessThan(later - 4);
	expect(await time(media)).toBeGreaterThan(later - 6);

	await page.keyboard.press('5');
	await expect.poll(() => time(media)).toBeGreaterThan(29);

	await page.keyboard.press('?');
	const dialog = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
	await expect(dialog).toBeVisible();
	await expect(dialog.getByText('Next video')).toBeVisible();

	// Keys stay in the dialog while it is open.
	await page.keyboard.press('k');
	await page.waitForTimeout(300);
	expect(await paused(media)).toBe(true);
	await page.keyboard.press('Escape');
	await expect(dialog).toBeHidden();

	await page.getByRole('heading', { level: 1, name: TITLE }).click();
	await page.keyboard.press('k');
	await expect.poll(() => paused(media)).toBe(false);
});

test('mute is remembered on this device', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await playing(page, media);
	await page.getByRole('heading', { level: 1, name: TITLE }).click();
	expect(await muted(media)).toBe(false);
	await page.keyboard.press('m');
	await expect.poll(() => muted(media)).toBe(true);

	await page.reload();
	await playing(page, media);
	await expect.poll(() => muted(media)).toBe(true);
});

test('loop repeats the video and the sleep timer shows its choice', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await playing(page, media);

	await openSettings(page);
	const loop = page.getByTestId('player-loop');
	await expect(loop).toHaveAttribute('aria-checked', 'false');
	await loop.click();
	await expect(loop).toHaveAttribute('aria-checked', 'true');
	await expect(loop).toContainText('On');

	await page.getByTestId('player-sleep').click();
	await page.getByTestId('player-sleep-30').click();
	await openSettings(page);
	await expect(page.getByTestId('player-sleep')).toContainText('30 min');
	await page.keyboard.press('Escape');

	await media.evaluate((el) => ((el as Media).currentTime = 58.5));
	await expect.poll(() => time(media), { timeout: 10_000 }).toBeLessThan(5);
	expect(await paused(media)).toBe(false);
});

test('the media session shows the video and its controls', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await playing(page, media);

	const session = () =>
		page.evaluate(() => ({
			title: navigator.mediaSession.metadata?.title,
			artist: navigator.mediaSession.metadata?.artist,
			state: navigator.mediaSession.playbackState
		}));
	await expect.poll(session).toEqual({ title: TITLE, artist: 'Northbound', state: 'playing' });

	await page.getByRole('heading', { level: 1, name: TITLE }).click();
	await page.keyboard.press('k');
	await expect.poll(async () => (await session()).state).toBe('paused');
});
