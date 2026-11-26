import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);
const thumbs = (id: string) => [{ url: `/proxy/image/${id}`, width: 405, height: 720 }];
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

const short = (n: number) =>
	video(n, { title: `Forge short ${n}`, is_short: true, duration_seconds: null, channel: null });

function json(route: Route, body: unknown, status = 200) {
	return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

/** Two pages per type; records the type and continuation of every request. */
async function mockSearch(page: Page) {
	const requests: string[] = [];
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
	await page.route('**/api/v0/search?*', (route) => {
		const url = new URL(route.request().url());
		const type = url.searchParams.get('type') ?? 'all';
		const continuation = url.searchParams.get('continuation');
		requests.push(`${type}:${continuation ?? ''}`);
		const pages: Record<string, unknown[][]> = {
			all: [[video(1), { type: 'channel', ...forge, handle: '@forge', description: '' }], []],
			videos: [[video(1), video(2)], [video(3)]],
			shorts: [
				[short(11), short(12)],
				[short(13), short(14)]
			],
			channels: [
				[{ type: 'channel', ...forge, handle: '@hammerandtongs', description: 'Blacksmithing.' }],
				[]
			],
			playlists: [[], []]
		};
		const [first, second] = pages[type];
		return json(
			route,
			continuation
				? { items: second, continuation: null }
				: { items: first, continuation: `${type}-2` }
		);
	});
	return requests;
}

test('search tabs switch the result type and keep it in the URL', async ({ page }) => {
	const requests = await mockSearch(page);
	await page.goto('/results?search_query=forge');

	const tabs = page.getByRole('navigation', { name: 'Result types' });
	await expect(tabs.getByRole('link', { name: 'All' })).toHaveAttribute('aria-current', 'page');

	await tabs.getByRole('link', { name: 'Shorts' }).click();
	await expect(page).toHaveURL('/results?search_query=forge&type=shorts');
	await expect(tabs.getByRole('link', { name: 'Shorts' })).toHaveAttribute('aria-current', 'page');
	const results = page.getByRole('list', { name: 'Search results' });
	await expect(results.getByRole('link', { name: 'Forge short 11' })).toHaveAttribute(
		'href',
		'/shorts/vid0000011A'
	);
	await page.getByText('End of results').scrollIntoViewIfNeeded();
	await expect(results.getByRole('link', { name: 'Forge short 14' })).toBeVisible();

	await tabs.getByRole('link', { name: 'Channels' }).click();
	await expect(page).toHaveURL('/results?search_query=forge&type=channels');
	await expect(results.getByRole('link', { name: 'Hammer & Tongs', exact: true })).toHaveAttribute(
		'href',
		`/channel/${forge.id}`
	);
	await expect(results.getByTestId('subscribe-button')).toBeVisible();

	await tabs.getByRole('link', { name: 'Playlists' }).click();
	await expect(page.getByRole('heading', { name: 'No playlists for “forge”' })).toBeVisible();

	await expect
		.poll(() => requests)
		.toEqual(
			expect.arrayContaining([
				'all:',
				'shorts:',
				'shorts:shorts-2',
				'channels:',
				'playlists:',
				'playlists:playlists-2'
			])
		);
	// Continuations are requested with the type they came from.
	for (const request of requests) {
		const [type, continuation] = request.split(':');
		expect(continuation === '' || continuation.startsWith(`${type}-`)).toBe(true);
	}
});

test('an unknown type falls back to all results', async ({ page }) => {
	const requests = await mockSearch(page);
	await page.goto('/results?search_query=forge&type=movies');

	const tabs = page.getByRole('navigation', { name: 'Result types' });
	await expect(tabs.getByRole('link', { name: 'All' })).toHaveAttribute('aria-current', 'page');
	await expect(page.getByRole('link', { name: /Forge video 1/ })).toBeVisible();
	expect(requests[0]).toBe('all:');
});

test('results without a known channel render without channel links', async ({ page }) => {
	const noName = { id: '', name: '', avatar: [], verified: false, subscriber_count: null };
	const errors: Error[] = [];
	page.on('pageerror', (error) => errors.push(error));
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
	await page.route('**/api/v0/search?*', (route) => {
		const type = new URL(route.request().url()).searchParams.get('type');
		const items =
			type === 'shorts'
				? [short(21), video(22, { title: 'Forge short 22', is_short: true, channel: noName })]
				: [
						video(1),
						video(2, { channel: null }),
						video(3, { channel: noName }),
						{
							type: 'playlist',
							id: 'PLnochannel',
							title: 'No channel',
							thumbnails: [],
							channel: noName
						}
					];
		return json(route, { items, continuation: null });
	});

	await page.goto('/results?search_query=forge&type=videos');
	const results = page.getByRole('list', { name: 'Search results' });
	await expect(results.getByRole('link', { name: /Forge video 3/ })).toBeVisible();
	await expect(results.getByRole('link', { name: /No channel/ })).toBeVisible();
	await expect(results.locator('a[href^="/channel/"]')).toHaveCount(1);

	await page.getByRole('link', { name: 'Shorts' }).click();
	await expect(results.getByRole('link', { name: 'Forge short 22' })).toHaveAttribute(
		'href',
		'/shorts/vid0000022A'
	);
	expect(errors).toEqual([]);
});

test('playlist results link to the playlist page', async ({ page }) => {
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
	await page.route('**/api/v0/search?*', (route) =>
		json(route, {
			items: [
				{
					type: 'playlist',
					id: 'PLforgebasics',
					title: 'Forge basics',
					thumbnails: thumbs('pl'),
					channel: forge,
					video_count: 24,
					is_course: false
				},
				{
					type: 'playlist',
					id: 'PLforgecourse',
					title: 'Knife making course',
					thumbnails: thumbs('course'),
					channel: forge,
					video_count: 12,
					is_course: true
				}
			],
			continuation: null
		})
	);
	await page.goto('/results?search_query=forge&type=playlists');

	await expect(page.getByRole('link', { name: /Forge basics/ })).toHaveAttribute(
		'href',
		'/playlist?list=PLforgebasics'
	);
	const cards = page.getByRole('listitem');
	await expect(
		cards.filter({ hasText: 'Knife making course' }).getByText('Course', { exact: true })
	).toBeVisible();
	await expect(
		cards.filter({ hasText: 'Forge basics' }).getByText('Course', { exact: true })
	).toHaveCount(0);
	await expect(page.getByText('End of results')).toBeVisible();
});

test('a failed tab shows the error state', async ({ page }) => {
	await page.route('**/api/v0/search?*', (route) =>
		json(route, { code: 'upstream_error', message: 'boom' }, 502)
	);
	await page.goto('/results?search_query=forge&type=videos');
	await expect(page.getByRole('alert')).toContainText('Search failed', { timeout: 10_000 });
	await expect(page.getByRole('button', { name: 'Try again' })).toBeVisible();
});
