import { expect, test, type Page, type Route } from '@playwright/test';
import { readFileSync } from 'node:fs';

test.use({ locale: 'en-US' });

const ID = 'sbk0000001A';

// The 6 s clip of the shorts tests, played on the watch page.
const CLIP = readFileSync(new URL('./fixtures/short.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/short.mpd', import.meta.url), 'utf8').replace(
	'{id}',
	ID
);

const segments = [
	{ id: 'sp', category: 'sponsor', action: 'skip', start: 1, end: 3 },
	{ id: 'sf', category: 'selfpromo', action: 'skip', start: 3.5, end: 5.5 }
];

function json(route: Route, body: unknown, status = 200) {
	return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

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

/** A guest on an instance with SponsorBlock; nothing reaches the backend or YouTube. */
async function mockApi(page: Page) {
	const requests: string[] = [];
	page.on('request', (request) => requests.push(request.url()));
	await page.route('**/api/v0/**', (route) => json(route, { code: 'not_found', message: '' }, 404));
	await page.route('**/proxy/**', (route) => route.fulfill({ status: 404, body: '' }));
	await page.route('**/proxy/stream/**', serveClip);
	await page.route('**/api/v0/instance', (route) =>
		json(route, {
			name: 'Oxistream',
			version: '0.1.0',
			auth: {
				local_enabled: true,
				registration_enabled: true,
				require_login: false,
				oidc_enabled: false,
				oidc_name: null
			},
			sponsorblock: true
		})
	);
	await page.route('**/api/v0/auth/me', (route) =>
		json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route(`**/api/v0/videos/${ID}/manifest.mpd`, (route) =>
		route.fulfill({ status: 200, contentType: 'application/dash+xml', body: MPD })
	);
	await page.route(`**/api/v0/videos/${ID}/segments`, (route) => json(route, { segments }));
	await page.route(`**/api/v0/videos/${ID}`, (route) =>
		json(route, {
			id: ID,
			title: 'Sponsored clip',
			description: '',
			channel: { id: 'UCforge', name: 'Hammer & Tongs', avatar: [], verified: false },
			view_count: 10,
			like_count: null,
			published_at: null,
			published_text: null,
			duration_seconds: 6,
			is_live: false,
			keywords: [],
			thumbnails: [],
			subtitles: [],
			related: []
		})
	);
	return requests;
}

const media = (page: Page) => page.locator('dash-video');
const currentTime = (page: Page) =>
	media(page).evaluate((element) => (element as unknown as HTMLMediaElement).currentTime);

test('sponsors are skipped automatically, with undo', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	await expect(page.locator('.oxi-segment-marker')).toHaveCount(2);

	const skipped = page.getByTestId('segment-skipped');
	await expect(skipped).toContainText('Skipped: Sponsor', { timeout: 10_000 });
	expect(await currentTime(page)).toBeGreaterThanOrEqual(2.9);

	await media(page).evaluate((element) => (element as unknown as HTMLMediaElement).pause());
	await skipped.getByRole('button', { name: 'Undo' }).click();
	await expect(skipped).toHaveCount(0);
	await expect.poll(() => currentTime(page)).toBeLessThan(1.5);
	// After an undo, the sponsor plays and offers its button instead.
	await media(page).evaluate((element) => (element as unknown as HTMLMediaElement).play());
	await expect(page.getByTestId('segment-skip')).toHaveText(/Skip Sponsor/, { timeout: 5_000 });
	await expect(skipped).toHaveCount(0);

	expect(requests.some((url) => url.includes('sponsor.ajay.app'))).toBe(false);
});

test('other segments offer a skip button', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);

	const button = page.getByTestId('segment-skip');
	await expect(button).toHaveText(/Skip Self-promotion/, { timeout: 10_000 });
	await media(page).evaluate((element) => (element as unknown as HTMLMediaElement).pause());
	await button.click();
	await expect.poll(() => currentTime(page)).toBeGreaterThanOrEqual(5.4);
	await expect(button).toHaveCount(0);
});

test('guests change preferences from the player menu', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);
	await expect(page.locator('.oxi-segment-marker')).toHaveCount(2);

	await page.locator('.video-controls-settings-button').click();
	await page.getByTestId('player-sponsorblock-settings').click();
	const dialog = page.getByRole('dialog', { name: 'SponsorBlock' });
	await expect(dialog).toBeVisible();
	await expect(dialog.getByText('Saved in this browser.')).toBeVisible();

	await dialog.locator('[data-category="sponsor"]').getByText('Off').click();
	await expect(page.locator('.oxi-segment-marker')).toHaveCount(1);
	const stored = await page.evaluate(() => localStorage.getItem('oxistream:sponsorblock'));
	const parsed = JSON.parse(stored!) as { categories: Record<string, string> };
	expect(parsed.categories.sponsor).toBe('off');

	await page.reload();
	await expect(page.locator('.oxi-segment-marker')).toHaveCount(1);
});
