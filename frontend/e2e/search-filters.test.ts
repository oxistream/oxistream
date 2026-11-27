import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);
const thumbs = (id: string) => [{ url: `/proxy/image/${id}`, width: 640, height: 360 }];
const forge = {
	id: 'UCm0ck4nv1lF0rg3H4mm3r1',
	name: 'Hammer & Tongs',
	avatar: thumbs('forge-avatar'),
	verified: true,
	subscriber_count: 284000
};

function video(n: number, extra: Record<string, unknown> = {}) {
	return {
		type: 'video',
		id: `vid${String(n).padStart(7, '0')}A`,
		title: `Forge video ${n}`,
		thumbnails: thumbs(`v${n}`),
		duration_seconds: 600 + n,
		view_count: 1000 * n,
		published_at: '2026-11-20T10:00:00Z',
		published_text: '6 days ago',
		is_live: false,
		is_short: false,
		is_upcoming: false,
		channel: forge,
		description: null,
		...extra
	};
}

function json(route: Route, body: unknown, status = 200) {
	return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

/** Records the filter query of every search request. */
async function mockSearch(page: Page) {
	const requests: string[] = [];
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
	await page.route('**/api/v0/search?*', (route) => {
		const params = new URL(route.request().url()).searchParams;
		params.delete('q');
		requests.push(params.toString());
		const live = video(9, { title: 'Forge live', is_live: true, duration_seconds: null });
		return json(route, {
			items: [
				video(1),
				live,
				{ type: 'channel', ...forge, handle: '@forge', description: 'Blacksmithing.' }
			],
			continuation: null
		});
	});
	return requests;
}

test('filters are sent to the API and kept in the URL', async ({ page }) => {
	const requests = await mockSearch(page);
	await page.goto('/results?search_query=forge');
	await expect(page.getByRole('link', { name: /Forge video 1/ })).toBeVisible();

	const filters = page.getByRole('group', { name: 'Filters' });
	await filters.getByRole('button', { name: 'Upload date: Any time' }).click();
	await page.getByRole('menuitemradio', { name: 'This week' }).click();
	await expect(page).toHaveURL('/results?search_query=forge&upload_date=week');

	await filters.getByRole('button', { name: 'Sort by: Relevance' }).click();
	await page.getByRole('menuitemradio', { name: 'Most views' }).click();
	await expect(page).toHaveURL('/results?search_query=forge&upload_date=week&sort=views');
	await expect(filters.getByRole('button', { name: 'Sort by: Most views' })).toBeVisible();

	await page.reload();
	await expect(filters.getByRole('button', { name: 'Upload date: This week' })).toBeVisible();
	await expect(filters.getByRole('button', { name: 'Sort by: Most views' })).toBeVisible();

	await page.goBack();
	await expect(page).toHaveURL('/results?search_query=forge&upload_date=week');
	await expect(filters.getByRole('button', { name: 'Sort by: Relevance' })).toBeVisible();

	await filters.getByRole('button', { name: 'Clear filters' }).click();
	await expect(page).toHaveURL('/results?search_query=forge');
	await expect(filters.getByRole('button', { name: 'Clear filters' })).toHaveCount(0);

	await expect
		.poll(() => requests)
		.toEqual(expect.arrayContaining(['', 'upload_date=week', 'upload_date=week&sort=views']));
});

test('tabs keep the filters they offer', async ({ page }) => {
	const requests = await mockSearch(page);
	await page.goto('/results?search_query=forge&duration=long&sort=date&upload_date=bogus');

	const tabs = page.getByRole('navigation', { name: 'Result types' });
	await expect(tabs.getByRole('link', { name: 'Videos' })).toHaveAttribute(
		'href',
		'/results?search_query=forge&type=videos&duration=long&sort=date'
	);
	await expect(tabs.getByRole('link', { name: 'Shorts' })).toHaveAttribute(
		'href',
		'/results?search_query=forge&type=shorts&sort=date'
	);
	await expect(tabs.getByRole('link', { name: 'Channels' })).toHaveAttribute(
		'href',
		'/results?search_query=forge&type=channels'
	);

	await tabs.getByRole('link', { name: 'Channels' }).click();
	await expect(page.getByRole('group', { name: 'Filters' })).toHaveCount(0);
	await expect.poll(() => requests).toEqual(['duration=long&sort=date', 'type=channels']);
});

test('live results are marked and not linked to the player', async ({ page }) => {
	await mockSearch(page);
	await page.goto('/results?search_query=forge');

	const results = page.getByRole('list', { name: 'Search results' });
	const live = results.getByRole('listitem').filter({ hasText: 'Forge live' });
	await expect(live.getByText('Live', { exact: true })).toBeVisible();
	await expect(live.locator('a[href^="/watch"]')).toHaveCount(0);
	await expect(results.locator('a[href="/watch?v=vid0000001A"]')).toHaveCount(1);

	const channel = results.getByTestId('channel-result');
	await expect(channel.getByRole('link', { name: 'Hammer & Tongs' })).toHaveAttribute(
		'href',
		`/channel/${forge.id}`
	);
	const box = await channel.boundingBox();
	expect(box?.height).toBeLessThan(120);
});
