import { readFileSync } from 'node:fs';
import { expect, test, type Locator, type Page, type Route } from '@playwright/test';

test.use({
	locale: 'en-US',
	launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] }
});

const ID = 'syncMini0A8';
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

const time = (media: Locator) => media.evaluate((el) => (el as HTMLMediaElement).currentTime);
const paused = (media: Locator) => media.evaluate((el) => (el as HTMLMediaElement).paused);
const sameMedia = (media: Locator) =>
	media.evaluate((el) => el === (window as { oxiMedia?: Element }).oxiMedia);

async function goHome(page: Page) {
	await page.getByRole('link', { name: 'Home', exact: true }).filter({ visible: true }).click();
	await expect(page).toHaveURL('/');
}

test('playback carries on in the mini player and back on the watch page', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);

	const media = page.locator('dash-video');
	const slot = page.getByTestId('player-slot');
	await expect(slot).toBeVisible();
	await expect.poll(() => time(media), { timeout: 15_000 }).toBeGreaterThan(1);
	await media.evaluate((el) => ((window as { oxiMedia?: Element }).oxiMedia = el));

	// The player sits exactly over the slot on the watch page.
	const slotBox = (await slot.boundingBox())!;
	const playerBox = (await page.getByTestId('app-player').boundingBox())!;
	expect(Math.abs(playerBox.x - slotBox.x)).toBeLessThan(1);
	expect(Math.abs(playerBox.y - slotBox.y)).toBeLessThan(1);
	expect(Math.abs(playerBox.width - slotBox.width)).toBeLessThan(1);

	await goHome(page);
	const mini = page.getByRole('region', { name: 'Mini player' });
	await expect(mini).toBeVisible();
	await expect(mini.getByTestId('mini-player-title')).toHaveText(TITLE);
	await expect(mini.getByText('Northbound')).toBeVisible();
	expect(await sameMedia(media)).toBe(true);
	const away = await time(media);
	expect(away).toBeGreaterThan(1);
	await expect.poll(() => time(media)).toBeGreaterThan(away + 0.5);

	await mini.getByRole('button', { name: 'Pause', exact: true }).click();
	await expect.poll(() => paused(media)).toBe(true);
	await mini.getByRole('button', { name: 'Play', exact: true }).click();
	await expect.poll(() => paused(media)).toBe(false);

	const beforeReturn = await time(media);
	await mini.getByTestId('mini-player-title').click();
	await expect(page).toHaveURL(`/watch?v=${ID}`);
	await expect(page.getByRole('heading', { level: 1, name: TITLE })).toBeVisible();
	await expect(mini).toHaveCount(0);
	await expect(page.getByTestId('player-slot')).toBeVisible();
	expect(await sameMedia(media)).toBe(true);
	expect(await time(media)).toBeGreaterThanOrEqual(beforeReturn);

	await goHome(page);
	await expect(mini).toBeVisible();
	await mini.getByRole('button', { name: 'Close player' }).click();
	await expect(mini).toHaveCount(0);
	await expect(media).toHaveCount(0);
});

test('the mini player is a slim card along the bottom on phones', async ({ page }) => {
	await page.setViewportSize({ width: 390, height: 844 });
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await expect.poll(() => time(media), { timeout: 15_000 }).toBeGreaterThan(0.5);

	// The logo in the header leads home on phones.
	await goHome(page);
	const mini = page.getByRole('region', { name: 'Mini player' });
	await expect(mini).toBeVisible();
	const box = (await mini.boundingBox())!;
	expect(box.width).toBeGreaterThan(360);
	expect(box.height).toBeLessThan(100);
	expect(box.y + box.height).toBeGreaterThan(844 - 20);
});
