import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);
const ID = 'UCm0ck4nv1lF0rg3H4mm3r1';
const wide = (id: string) => [{ url: `/proxy/image/${id}-wide`, width: 480, height: 270 }];

function video(n: number, extra: Record<string, unknown> = {}) {
	return {
		id: `vid${String(n).padStart(7, '0')}A`,
		title: `Forge video ${n}`,
		thumbnails: wide(`v${n}`),
		duration_seconds: 600 + n,
		view_count: 1000 * n,
		viewers: null,
		published_at: '2026-11-20T10:00:00Z',
		published_text: '6 days ago',
		is_live: false,
		is_short: false,
		is_upcoming: false,
		channel: null,
		description: null,
		...extra
	};
}

function short(n: number) {
	return video(n, {
		id: `sho${String(n).padStart(7, '0')}A`,
		title: `Forge short ${n}`,
		duration_seconds: null,
		published_at: null,
		published_text: null,
		is_short: true,
		thumbnails: [...wide(`s${n}`), { url: `/proxy/image/s${n}-tall`, width: 404, height: 720 }]
	});
}

function json(route: Route, body: unknown, status = 200) {
	return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

/** A channel with shorts but no livestreams; records the query of every tab request. */
async function mockChannel(page: Page, { playlists = true } = {}) {
	const requests: string[] = [];
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
	await page.route('**/api/v0/resolve?*', (route) => json(route, { kind: 'channel', id: ID }));
	await page.route(`**/api/v0/channels/${ID}`, (route) =>
		json(route, {
			id: ID,
			name: 'Hammer & Tongs',
			handle: '@hammerandtongs',
			avatar: wide('avatar'),
			verified: false,
			subscriber_count: 284000,
			video_count: 120,
			description: 'Traditional blacksmithing.',
			banner: [],
			tags: [],
			has_shorts: true,
			has_live: false,
			videos: { items: [1, 2, 3].map((n) => video(n)), continuation: null }
		})
	);
	await page.route(`**/api/v0/channels/${ID}/*`, (route) => {
		const url = new URL(route.request().url());
		const tab = url.pathname.split('/').pop();
		requests.push(`${tab}${url.search}`);
		const continuation = url.searchParams.get('continuation');
		switch (tab) {
			case 'videos':
				return json(route, {
					items: [7, 8].map((n) => video(n, { title: `Popular video ${n}` })),
					continuation: null
				});
			case 'shorts':
				return continuation
					? json(route, { items: [4, 5].map(short), continuation: null })
					: json(route, { items: [1, 2, 3].map(short), continuation: 'shorts-2' });
			case 'live':
				return json(route, { items: [], continuation: null });
			case 'playlists':
				return json(route, {
					items: playlists
						? [
								{
									id: 'PLforgebasics',
									title: 'Forge basics',
									thumbnails: wide('pl'),
									channel: null,
									video_count: 24,
									is_course: false
								},
								{
									id: 'PLforgecourse',
									title: 'Knife making course',
									thumbnails: wide('course'),
									channel: null,
									video_count: 12,
									is_course: true
								}
							]
						: [],
					continuation: null
				});
			case 'search':
				return json(route, {
					items: url.searchParams.get('q') === 'tongs' ? [video(9, { title: 'Making tongs' })] : [],
					continuation: null
				});
		}
		return json(route, { code: 'not_found', message: 'not found' }, 404);
	});
	return requests;
}

test('tabs follow YouTube URLs and hide tabs the channel lacks', async ({ page }) => {
	await mockChannel(page);
	await page.goto('/@hammerandtongs');

	const tabs = page.getByRole('navigation', { name: 'Channel sections' });
	await expect(tabs.getByRole('link')).toHaveText(['Videos', 'Shorts', 'Playlists']);
	await expect(tabs.getByRole('link', { name: 'Videos' })).toHaveAttribute('aria-current', 'page');
	await expect(page.getByRole('list', { name: 'Videos' }).getByRole('listitem')).toHaveCount(3);

	await tabs.getByRole('link', { name: 'Shorts' }).click();
	await expect(page).toHaveURL(/\/@hammerandtongs\/shorts$/);
	await expect(page).toHaveTitle('Shorts · Hammer & Tongs · Oxistream');
	await expect(page.getByRole('heading', { level: 1, name: /Hammer & Tongs/ })).toBeVisible();

	// A tab the channel does not list still opens directly, with an empty state.
	await page.goto(`/channel/${ID}/streams`);
	await expect(tabs.getByRole('link', { name: 'Live' })).toHaveAttribute('aria-current', 'page');
	await expect(page.getByRole('heading', { name: 'No livestreams' })).toBeVisible();
});

test('shorts show portrait cards that open the shorts route and load more', async ({ page }) => {
	const requests = await mockChannel(page);
	await page.goto('/@hammerandtongs/shorts');

	const shorts = page.getByRole('list', { name: 'Shorts' });
	const first = shorts.getByRole('link', { name: 'Forge short 1' });
	await expect(first).toHaveAttribute('href', '/shorts/sho0000001A');
	await expect(shorts.locator('img').first()).toHaveAttribute('src', '/proxy/image/s1-tall');
	const box = await shorts.locator('img').first().boundingBox();
	expect(box!.height / box!.width).toBeCloseTo(16 / 9, 1);

	await expect(shorts.getByRole('listitem')).toHaveCount(5);
	expect(requests).toEqual(['shorts?sort=latest', 'shorts?continuation=shorts-2']);
});

test('the sort chips reload the tab in the chosen order', async ({ page }) => {
	const requests = await mockChannel(page);
	await page.goto('/@hammerandtongs/videos');

	const sort = page.getByRole('list', { name: 'Sort by' });
	await expect(sort.getByRole('link', { name: 'Latest' })).toHaveAttribute('aria-current', 'true');
	// The latest videos come with the channel itself.
	expect(requests).toEqual([]);

	await sort.getByRole('link', { name: 'Popular' }).click();
	await expect(page).toHaveURL(/\/@hammerandtongs\/videos\?sort=popular$/);
	await expect(sort.getByRole('link', { name: 'Popular' })).toHaveAttribute('aria-current', 'true');
	await expect(page.getByRole('link', { name: /Popular video 7/ })).toBeVisible();
	expect(requests).toEqual(['videos?sort=popular']);
});

test('playlists link to the YouTube playlist view', async ({ page }) => {
	await mockChannel(page);
	await page.goto('/@hammerandtongs/playlists');

	const playlists = page.getByRole('list', { name: 'Playlists' });
	await expect(playlists.getByRole('link', { name: /Forge basics/ })).toHaveAttribute(
		'href',
		'/playlist?list=PLforgebasics'
	);
	await expect(playlists.getByText('24 videos', { exact: true })).toBeVisible();

	const cards = playlists.getByRole('listitem');
	await expect(
		cards.filter({ hasText: 'Knife making course' }).getByText('Course', { exact: true })
	).toBeVisible();
	await expect(cards.filter({ hasText: 'Forge basics' }).getByText('Course')).toHaveCount(0);
});

test('a channel without playlists shows an empty state', async ({ page }) => {
	await mockChannel(page, { playlists: false });
	await page.goto(`/channel/${ID}/playlists`);
	await expect(page.getByRole('heading', { name: 'No playlists' })).toBeVisible();
});

test('the magnifier opens a channel search with its own URL', async ({ page }) => {
	const requests = await mockChannel(page);
	await page.goto('/@hammerandtongs');

	await page.getByRole('button', { name: 'Search this channel' }).click();
	const input = page.getByRole('searchbox', { name: 'Search this channel' });
	await expect(input).toBeFocused();
	await input.fill('tongs');
	await input.press('Enter');

	await expect(page).toHaveURL(/\/@hammerandtongs\/search\?query=tongs$/);
	await expect(page).toHaveTitle('Results for “tongs” · Hammer & Tongs · Oxistream');
	await expect(
		page.getByRole('list', { name: 'Search results' }).getByRole('link', { name: /Making tongs/ })
	).toBeVisible();
	expect(requests).toEqual(['search?q=tongs']);

	await input.fill('anvil');
	await input.press('Enter');
	await expect(page.getByRole('heading', { name: 'No results for “anvil”' })).toBeVisible();

	// Escape closes an empty field on the other tabs.
	await page.goto('/@hammerandtongs/shorts');
	await page.getByRole('button', { name: 'Search this channel' }).click();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('button', { name: 'Search this channel' })).toBeVisible();
});
