import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);
const CHANNEL = 'UCm0ck4nv1lF0rg3H4mm3r1';
const shortId = (n: number) => `sho${String(n).padStart(7, '0')}A`;

const MPD = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011" type="static" mediaPresentationDuration="PT30S" minBufferTime="PT1.5S"><Period></Period></MPD>`;

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
	await page.route('**/proxy/stream/**', (route) => route.fulfill({ status: 404, body: '' }));
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({ status: 200, contentType: 'application/dash+xml', body: MPD })
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

test('arrow keys and j/k move between the channel shorts', async ({ page }) => {
	await mockApi(page);
	await page.goto('/');
	await page.goto(`/shorts/${shortId(2)}`);

	await expect(viewer(page).getByRole('heading', { name: 'Forge short 2' })).toBeVisible();
	await expect(page).toHaveTitle(/Forge short 2/);
	await expect(viewer(page).getByTestId('short-watch-link')).toHaveAttribute(
		'href',
		`/watch?v=${shortId(2)}`
	);

	await page.keyboard.press('ArrowDown');
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(3)}$`));
	await expect(viewer(page).getByRole('heading', { name: 'Forge short 3' })).toBeVisible();
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

test('a short without a channel plays on its own', async ({ page }) => {
	await mockApi(page, { channelId: '' });
	await page.goto(`/shorts/${shortId(1)}`);

	await expect(viewer(page).getByRole('heading', { name: 'Forge short 1' })).toBeVisible();
	await expect(viewer(page).getByTestId('subscribe-button')).toHaveCount(0);
	await expect(page.getByTestId('shorts-next')).toBeDisabled();
	await page.keyboard.press('ArrowDown');
	await expect(page).toHaveURL(new RegExp(`/shorts/${shortId(1)}$`));
});
