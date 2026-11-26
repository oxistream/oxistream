import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);
const thumbs = (id: string) => [
	{ url: `/proxy/image/${id}-360`, width: 360, height: 202 },
	{ url: `/proxy/image/${id}-720`, width: 720, height: 404 }
];
const forge = {
	id: 'UCm0ck4nv1lF0rg3H4mm3r1',
	name: 'Hammer & Tongs',
	avatar: thumbs('forge-avatar'),
	verified: true,
	subscriber_count: null
};

function video(n: number, extra: Record<string, unknown> = {}) {
	return {
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
		description: `Description of video ${n}`,
		...extra
	};
}

function json(route: Route, body: unknown, status = 200) {
	return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

async function mockImages(page: Page) {
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
}

/** Two search pages; records every continuation the app asks for. */
async function mockSearch(page: Page) {
	const continuations: (string | null)[] = [];
	await page.route('**/api/v0/search?*', (route) => {
		const url = new URL(route.request().url());
		const continuation = url.searchParams.get('continuation');
		continuations.push(continuation);
		if (!continuation) {
			return json(route, {
				items: [
					{ type: 'video', ...video(1) },
					{
						type: 'channel',
						...forge,
						handle: '@hammerandtongs',
						subscriber_count: 284000,
						description: 'Traditional blacksmithing.'
					},
					{
						type: 'playlist',
						id: 'PLforgebasics',
						title: 'Forge basics',
						thumbnails: thumbs('pl'),
						channel: forge,
						video_count: 24
					},
					{ type: 'video', ...video(2, { is_live: true, duration_seconds: null }) },
					{ type: 'video', ...video(3) }
				],
				continuation: 'page-2'
			});
		}
		return json(route, {
			items: [4, 5, 6].map((n) => ({ type: 'video', ...video(n) })),
			continuation: null
		});
	});
	return continuations;
}

test.beforeEach(async ({ page }) => {
	await mockImages(page);
});

test('search renders videos, channels and playlists and loads more on scroll', async ({ page }) => {
	const continuations = await mockSearch(page);
	await page.goto('/results?search_query=forge');

	await expect(page).toHaveTitle('forge · Oxistream');
	await expect(page.getByTestId('search-input')).toHaveValue('forge');

	const results = page.getByRole('list', { name: 'Search results' });
	await expect(results.getByRole('link', { name: /Forge video 1/ })).toHaveAttribute(
		'href',
		'/watch?v=vid0000001A'
	);
	await expect(
		results.getByRole('link', { name: 'Hammer & Tongs', exact: true }).first()
	).toBeVisible();
	await expect(results.getByText('284K subscribers')).toBeVisible();
	await expect(results.getByRole('link', { name: 'Forge basics' })).toHaveAttribute(
		'href',
		'/playlist?list=PLforgebasics'
	);
	await expect(results.getByText('Live', { exact: true })).toBeVisible();

	await page.getByText('End of results').scrollIntoViewIfNeeded();
	await expect(results.getByRole('link', { name: /Forge video 6/ })).toBeVisible();
	await expect(page.getByText('End of results')).toBeVisible();
	expect(continuations).toEqual([null, 'page-2']);
});

test('load more button works where IntersectionObserver is missing', async ({ page }) => {
	await page.addInitScript(() => {
		// @ts-expect-error simulate an old browser
		delete window.IntersectionObserver;
	});
	await mockSearch(page);
	await page.goto('/results?search_query=forge');

	const results = page.getByRole('list', { name: 'Search results' });
	await expect(results.getByRole('listitem')).toHaveCount(5);
	await page.getByRole('button', { name: 'Load more' }).click();
	await expect(results.getByRole('listitem')).toHaveCount(8);
	await expect(page.getByRole('button', { name: 'Load more' })).toBeHidden();
});

test('search shows an empty state and an error state with retry', async ({ page }) => {
	let fail = true;
	await page.route('**/api/v0/search?*', (route) => {
		const q = new URL(route.request().url()).searchParams.get('q');
		if (q === 'broken' && fail) {
			return json(route, { code: 'upstream_error', message: 'boom' }, 502);
		}
		return json(route, { items: [] });
	});

	await page.goto('/results?search_query=nothing');
	await expect(page.getByRole('heading', { name: 'No results for “nothing”' })).toBeVisible();

	await page.goto('/results?search_query=broken');
	await expect(page.getByRole('alert')).toContainText('Search failed', { timeout: 10_000 });
	fail = false;
	await page.getByRole('button', { name: 'Try again' }).click();
	await expect(page.getByRole('heading', { name: 'No results for “broken”' })).toBeVisible();
});

test('suggestions are keyboard navigable and pick a search', async ({ page }) => {
	await mockSearch(page);
	await page.route('**/api/v0/search/suggestions?*', (route) =>
		json(route, ['forge welding', 'forge build', 'forging a knife'])
	);
	await page.goto('/');

	const input = page.getByRole('combobox', { name: 'Search videos' });
	await input.click();
	await input.pressSequentially('forg');

	const listbox = page.getByRole('listbox', { name: 'Search suggestions' });
	await expect(listbox.getByRole('option')).toHaveCount(3);
	await expect(input).toHaveAttribute('aria-expanded', 'true');
	await expect(input).toHaveAttribute('aria-controls', (await listbox.getAttribute('id'))!);

	await input.press('ArrowDown');
	await input.press('ArrowDown');
	const second = listbox.getByRole('option', { name: 'forge build' });
	await expect(second).toHaveAttribute('aria-selected', 'true');
	await expect(input).toHaveAttribute('aria-activedescendant', (await second.getAttribute('id'))!);

	await input.press('Escape');
	await expect(listbox).toBeHidden();
	await expect(input).toHaveAttribute('aria-expanded', 'false');

	await input.press('ArrowDown');
	await expect(listbox).toBeVisible();
	await input.press('ArrowDown');
	await input.press('Enter');

	await expect(page).toHaveURL(/\/results\?search_query=forge\+build$/);
	await expect(input).toHaveValue('forge build');
	await expect(listbox).toBeHidden();
});

test('a pasted YouTube link opens its target directly', async ({ page }) => {
	await page.route('**/api/v0/resolve?*', (route) => {
		const url = new URL(route.request().url()).searchParams.get('url') ?? '';
		if (url.includes('@hammerandtongs')) return json(route, { kind: 'channel', id: forge.id });
		return json(route, { kind: 'video', id: 'dQw4w9WgXcQ', start_seconds: 42 });
	});
	await page.goto('/');

	const input = page.getByTestId('search-input');
	await input.fill('https://youtu.be/dQw4w9WgXcQ?t=42');
	await input.press('Enter');
	await expect(page).toHaveURL(/\/watch\?v=dQw4w9WgXcQ&t=42$/);

	await input.fill('youtube.com/@hammerandtongs');
	await input.press('Enter');
	await expect(page).toHaveURL(new RegExp(`/channel/${forge.id}$`));
});

async function mockChannel(page: Page) {
	await page.route(`**/api/v0/channels/${forge.id}`, (route) =>
		json(route, {
			...forge,
			handle: '@hammerandtongs',
			subscriber_count: 284000,
			video_count: 120,
			description: 'Traditional blacksmithing from a one-person forge.\n\nNew videos every week.',
			banner: [{ url: '/proxy/image/banner', width: 1707, height: 283 }],
			tags: [],
			videos: {
				items: [1, 2, 3].map((n) => video(n, { channel: null, view_count: null })),
				continuation: 'videos-2'
			}
		})
	);
	await page.route(`**/api/v0/channels/${forge.id}/videos?*`, (route) =>
		json(route, { items: [4, 5].map((n) => video(n, { channel: null })), continuation: null })
	);
	await page.route('**/api/v0/channels/UCmissing*', (route) =>
		json(route, { code: 'not_found', message: 'not found' }, 404)
	);
}

test('channel page renders its header and pages through videos', async ({ page }) => {
	await mockChannel(page);
	await page.goto(`/channel/${forge.id}`);

	await expect(page).toHaveTitle('Hammer & Tongs · Oxistream');
	await expect(page.getByRole('heading', { level: 1, name: /Hammer & Tongs/ })).toBeVisible();
	await expect(page.getByText('@hammerandtongs')).toBeVisible();
	await expect(page.getByText('284K subscribers')).toBeVisible();
	await expect(page.getByRole('img', { name: 'Verified' }).first()).toBeVisible();

	// Anonymous users are sent to sign-in and come back here afterwards.
	await expect(page.getByTestId('subscribe-button')).toHaveAttribute(
		'href',
		`/login?redirect=%2Fchannel%2F${forge.id}`
	);

	const videos = page.getByRole('list', { name: 'Videos' });
	await expect(videos.getByRole('listitem')).toHaveCount(5);
	await expect(videos.getByRole('link', { name: /Forge video 5/ })).toBeVisible();
});

test('a handle URL resolves and keeps its address', async ({ page }) => {
	await mockChannel(page);
	await page.route('**/api/v0/resolve?*', (route) =>
		json(route, { kind: 'channel', id: forge.id })
	);
	await page.goto('/@hammerandtongs');

	await expect(page.getByRole('heading', { level: 1, name: /Hammer & Tongs/ })).toBeVisible();
	await expect(page).toHaveURL(/\/@hammerandtongs$/);
});

test('an unknown channel shows a not-found state', async ({ page }) => {
	await mockChannel(page);
	await page.goto('/channel/UCmissing0000000000000');
	await expect(page.getByRole('heading', { name: 'Channel not found' })).toBeVisible();
	await expect(page).toHaveTitle('Channel not found · Oxistream');
});

test('shorts and short links redirect to the watch page', async ({ page }) => {
	await page.goto('/shorts/dQw4w9WgXcQ');
	await expect(page).toHaveURL(/\/watch\?v=dQw4w9WgXcQ$/);

	await page.goto('/dQw4w9WgXcQ?t=5');
	await expect(page).toHaveURL(/\/watch\?v=dQw4w9WgXcQ&t=5$/);
});

test('app routes are not taken for video ids or handles', async ({ page }) => {
	await page.goto('/settings');
	await expect(page).toHaveURL(/\/settings$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();

	await page.goto('/playlist?list=PLforgebasics');
	await expect(page.getByRole('heading', { level: 1, name: 'Playlist' })).toBeVisible();
	await expect(page.getByText('Playlists from YouTube arrive in a later release.')).toBeVisible();
});
