import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const LIST = 'PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI';
const PAGE_SIZES = [100, 100, 30];
const TOTAL = 232;
const MPD = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011" type="static" mediaPresentationDuration="PT60S" minBufferTime="PT1.5S"><Period></Period></MPD>`;

const videoId = (n: number) => `yt${String(n).padStart(8, '0')}x`;

function summary(n: number) {
	return {
		id: videoId(n),
		title: `Coast ${n}`,
		thumbnails: [],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: 300 + n,
		view_count: 1000,
		published_at: null,
		published_text: '2 years ago',
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

/** Item numbers (1-based) of each page. */
function pageItems(index: number) {
	const start = PAGE_SIZES.slice(0, index).reduce((sum, size) => sum + size, 0);
	return Array.from({ length: PAGE_SIZES[index] }, (_, i) => summary(start + i + 1));
}

const continuation = (index: number) => (index < PAGE_SIZES.length ? `page-${index}` : null);

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

/** A three-page YouTube playlist; `requests` records every playlist call in order. */
async function mockApi(page: Page) {
	const requests: string[] = [];
	await page.route('**/proxy/**', (route) => route.fulfill({ status: 404, body: '' }));
	await page.route('**/api/v0/**', (route) => json(route, { code: 'not_found', message: '' }, 404));
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
			}
		})
	);
	await page.route('**/api/v0/auth/me', (route) =>
		json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({ status: 200, contentType: 'application/dash+xml', body: MPD })
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		return json(route, {
			...summary(Number(id.slice(2, 10))),
			id,
			description: '',
			like_count: null,
			keywords: [],
			subtitles: [],
			related: []
		});
	});
	await page.route(/\/api\/v0\/youtube\/playlists\/[^/]+(\/videos)?(\?.*)?$/, (route) => {
		const url = new URL(route.request().url());
		const [, , , , , id, videos] = url.pathname.split('/');
		const token = url.searchParams.get('continuation');
		requests.push(videos ? `videos ${token}` : 'playlist');
		if (id !== LIST) return json(route, { code: 'not_found', message: 'not found' }, 404);
		if (videos) {
			const index = Number(token?.split('-')[1]);
			return json(route, { items: pageItems(index), continuation: continuation(index + 1) });
		}
		return json(route, {
			id: LIST,
			title: 'Coastal roads',
			description: 'Every drive along the coast',
			channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
			video_count: TOTAL,
			thumbnails: [],
			last_update: '2026-11-24',
			last_update_text: 'Last updated on Nov 24, 2026',
			videos: { items: pageItems(0), continuation: continuation(1) }
		});
	});
	return requests;
}

test('a YouTube playlist renders read-only and loads more on scroll', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/playlist?list=${LIST}`);

	await expect(page.getByRole('heading', { level: 1, name: 'Coastal roads' })).toBeVisible();
	await expect(page).toHaveTitle(/Coastal roads/);
	const hero = page.getByRole('complementary');
	await expect(hero.getByRole('link', { name: 'Northbound' })).toHaveAttribute(
		'href',
		'/channel/UCnorth'
	);
	await expect(hero.getByText('YouTube playlist')).toBeVisible();
	await expect(hero.getByText('232 videos', { exact: true }).last()).toBeVisible();
	await expect(hero.getByText(/^Updated /)).toBeVisible();
	await expect(page.getByRole('link', { name: 'Play all' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(1)}&list=${LIST}`
	);
	await expect(page.getByRole('button', { name: 'Edit playlist' })).toHaveCount(0);
	await expect(page.getByRole('button', { name: /^Reorder/ })).toHaveCount(0);
	await expect(page.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);

	const items = page.getByTestId('playlist-item');
	await expect(items).toHaveCount(100);
	await items.last().scrollIntoViewIfNeeded();
	await expect(items).toHaveCount(200);
	await items.last().scrollIntoViewIfNeeded();
	await expect(items).toHaveCount(230);
	await expect(page.getByText('Unavailable videos are hidden.')).toBeVisible();
	await expect(items.last().getByRole('link', { name: /Coast 230/ })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(230)}&list=${LIST}`
	);
	expect(requests).toEqual(['playlist', 'videos page-1', 'videos page-2']);
});

test('the watch page loads the next page and plays on into it', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/watch?v=${videoId(100)}&list=${LIST}`);

	const panel = page.getByTestId('playlist-panel');
	await expect(panel.getByText(`100 / ${TOTAL}`)).toBeVisible();
	await expect(panel.getByRole('link', { name: /Coast 100/ })).toHaveAttribute(
		'aria-current',
		'true'
	);
	await expect(panel.getByRole('link', { name: 'Next video' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(101)}&list=${LIST}`
	);
	expect(requests).toEqual(['playlist', 'videos page-1']);

	await page.locator('dash-video').dispatchEvent('ended');
	await expect(page).toHaveURL(`/watch?v=${videoId(101)}&list=${LIST}`);
	await expect(panel.getByText(`101 / ${TOTAL}`)).toBeVisible();
	await expect(panel.getByRole('link', { name: 'Previous video' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(100)}&list=${LIST}`
	);
});

test('a deep link finds its video on a later page', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/watch?v=${videoId(229)}&list=${LIST}`);

	const panel = page.getByTestId('playlist-panel');
	await expect(panel.getByText(`229 / 230`)).toBeVisible();
	await expect(panel.getByRole('link', { name: 'Next video' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(230)}&list=${LIST}`
	);
	expect(requests).toEqual(['playlist', 'videos page-1', 'videos page-2']);
});

test('mixes and unknown playlists get their own states', async ({ page }) => {
	const requests = await mockApi(page);

	await page.goto('/playlist?list=RDdQw4w9WgXcQ');
	await expect(
		page.getByRole('heading', { name: "Mixes can't be opened as playlists" })
	).toBeVisible();

	await page.goto(`/watch?v=${videoId(1)}&list=RDdQw4w9WgXcQ`);
	await expect(
		page.getByTestId('playlist-panel').getByText("Mixes can't be opened as playlists")
	).toBeVisible();
	expect(requests).toEqual([]);

	await page.goto('/playlist?list=PLzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz');
	await expect(page.getByRole('heading', { name: 'Playlist not found' })).toBeVisible();

	await page.goto(`/watch?v=${videoId(1)}&list=PLzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz`);
	await expect(page.getByTestId('playlist-panel').getByText('Playlist unavailable')).toBeVisible();
});
