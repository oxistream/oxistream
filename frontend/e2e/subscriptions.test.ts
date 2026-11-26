import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

const forge = {
	id: 'UCm0ck4nv1lF0rg3H4mm3r1',
	name: 'Hammer & Tongs',
	handle: '@hammerandtongs',
	avatar: [{ url: '/proxy/image/forge', width: 88, height: 88 }],
	subscriber_count: 284000,
	verified: true,
	subscribed_at: '2026-11-20T10:00:00Z'
};
const north = {
	...forge,
	id: 'UCm0ckN0rthb0und0R0ads2',
	name: 'Northbound',
	handle: '@northbound'
};
const pending = {
	...forge,
	id: 'UCm0ckP3nd1ngCh4nn3l000',
	name: null,
	handle: null,
	avatar: [],
	subscriber_count: null,
	verified: false
};

function feedVideo(n: number) {
	return {
		id: `feed${String(n).padStart(6, '0')}A`,
		title: `Feed video ${n}`,
		thumbnails: [{ url: `/proxy/image/v${n}`, width: 480, height: 270 }],
		duration_seconds: 600 + n,
		view_count: 1000 * n,
		published_at: '2026-11-20T10:00:00Z',
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false,
		channel: { id: forge.id, name: forge.name, avatar: forge.avatar, verified: true }
	};
}

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

/** A signed-in session whose subscriptions live in `state.list`. */
async function mockApi(page: Page, options: { signedIn?: boolean; list?: unknown[] } = {}) {
	const state = {
		list: [...(options.list ?? [forge, north])] as { id: string }[],
		/** Subscribe and unsubscribe requests, in order. */
		calls: [] as string[],
		feed: [] as (string | null)[]
	};
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
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
		options.signedIn === false
			? json(route, { code: 'unauthorized', message: '' }, 401)
			: json(route, account)
	);
	await page.route('**/api/v0/subscriptions', (route) => json(route, state.list));
	await page.route('**/api/v0/subscriptions/*', async (route) => {
		const method = route.request().method();
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		if (method === 'GET') return json(route, { subscribed: state.list.some((c) => c.id === id) });
		state.calls.push(`${method} ${id}`);
		if (method === 'PUT') {
			const channel = id === north.id ? north : { ...forge, id, name: 'Forge Two' };
			state.list.push(channel);
			return json(route, channel, 201);
		}
		state.list = state.list.filter((c) => c.id !== id);
		return route.fulfill({ status: 204 });
	});
	await page.route('**/api/v0/feed?*', (route) => {
		const continuation = new URL(route.request().url()).searchParams.get('continuation');
		state.feed.push(continuation);
		if (!continuation) {
			return json(route, {
				items: Array.from({ length: 6 }, (_, i) => feedVideo(i + 1)),
				continuation: 'next'
			});
		}
		return json(route, { items: [7, 8].map(feedVideo), continuation: null });
	});
	return state;
}

test('anonymous users get a sign-in prompt on the subscription pages', async ({ page }) => {
	await mockApi(page, { signedIn: false });

	await page.goto('/feed/subscriptions');
	await expect(page.getByRole('heading', { level: 1, name: 'Subscriptions' })).toBeVisible();
	await expect(page.getByRole('main').getByRole('link', { name: 'Sign in' })).toBeVisible();

	await page.goto('/feed/channels');
	await expect(page.getByRole('heading', { level: 1, name: 'Subscribed channels' })).toBeVisible();
	await expect(page.getByRole('main').getByRole('link', { name: 'Sign in' })).toBeVisible();
	await expect(page.getByTestId('import-export-link')).toHaveCount(0);
});

test('the feed lists videos of subscribed channels and loads more on scroll', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/subscriptions');

	const feed = page.getByRole('list', { name: 'Subscriptions' });
	await expect(feed.getByRole('link', { name: /Feed video 1/ })).toHaveAttribute(
		'href',
		'/watch?v=feed000001A'
	);
	await page.getByText("You're all caught up").scrollIntoViewIfNeeded();
	await expect(feed.getByRole('listitem')).toHaveCount(8);
	expect(state.feed).toEqual([null, 'next']);

	const sidebar = page.locator('[data-slot="sidebar"]');
	await expect(sidebar.getByRole('link', { name: 'Northbound' })).toBeVisible();
	await page.getByRole('main').getByRole('link', { name: 'Manage' }).click();
	await expect(page).toHaveURL(/\/feed\/channels$/);
});

test('without subscriptions the feed points to import and export', async ({ page }) => {
	await mockApi(page, { list: [] });
	await page.goto('/feed/subscriptions');

	await expect(page.getByRole('heading', { name: 'No subscriptions yet' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Import / export' })).toHaveAttribute(
		'href',
		'/subscriptions/import-export'
	);
});

test('subscribing on a channel page updates the button and the sidebar', async ({ page }) => {
	const state = await mockApi(page, { list: [north] });
	await page.route(`**/api/v0/channels/${forge.id}`, (route) =>
		json(route, {
			...forge,
			description: '',
			banner: [],
			tags: [],
			video_count: 1,
			videos: { items: [], continuation: null }
		})
	);
	await page.goto(`/channel/${forge.id}`);

	const button = page.getByTestId('subscribe-button');
	await expect(button).toHaveText('Subscribe');
	await button.click();
	await expect(button).toHaveText('Subscribed');
	const sidebar = page.locator('[data-slot="sidebar"]');
	await expect(sidebar.getByRole('link', { name: 'Forge Two' })).toBeVisible();

	await button.click();
	await expect(button).toHaveText('Subscribe');
	await expect(sidebar.getByRole('link', { name: 'Forge Two' })).toHaveCount(0);
	expect(state.calls).toEqual([`PUT ${forge.id}`, `DELETE ${forge.id}`]);
});

test('channels can be unsubscribed and resubscribed from the list', async ({ page }) => {
	const state = await mockApi(page, { list: [forge, north, pending] });
	await page.goto('/feed/channels');

	await expect(page.getByText('3 channels')).toBeVisible();
	await expect(page.getByRole('main').getByText('Fetching channel…')).toBeVisible();
	await expect(page.getByTestId('import-export-link')).toHaveAttribute(
		'href',
		'/subscriptions/import-export'
	);

	await page.getByRole('button', { name: 'Unsubscribe from Northbound' }).click();
	await expect(page.getByText('2 channels')).toBeVisible();
	const undo = page.getByRole('button', { name: 'Subscribe to Northbound again' });
	await expect(undo).toBeVisible();
	await undo.click();
	await expect(page.getByRole('button', { name: 'Unsubscribe from Northbound' })).toBeVisible();
	expect(state.calls).toEqual([`DELETE ${north.id}`, `PUT ${north.id}`]);
});

test('the collapsed sidebar shows channel avatars with tooltips', async ({ page }) => {
	const many = Array.from({ length: 9 }, (_, i) => ({
		...forge,
		id: `UCm0ckCh4nn3l${String(i).padStart(10, '0')}`,
		name: `Channel ${i + 1}`
	}));
	await mockApi(page, { list: [...many.slice(0, 2), pending, ...many.slice(2)] });
	await page.goto('/feed/channels');

	const sidebar = page.locator('[data-slot="sidebar"]');
	await expect(sidebar.getByRole('link', { name: 'Channel 1' })).toBeVisible();
	await page.keyboard.press('ControlOrMeta+b');
	await expect(page.locator('[data-slot="sidebar"][data-state="collapsed"]')).toBeVisible();

	const first = sidebar.getByRole('link', { name: 'Channel 1' });
	await expect(first).toHaveAttribute('href', `/channel/${many[0].id}`);
	await expect(first.locator('img')).toBeVisible();
	await expect(first).toHaveJSProperty('offsetWidth', 32);
	await expect(sidebar.getByRole('link', { name: 'All 10 channels' })).toHaveAttribute(
		'href',
		'/feed/channels'
	);
	await expect(sidebar.getByRole('link', { name: 'Channel 7' })).toHaveCount(0);

	await first.hover();
	await expect(page.locator('[data-slot="tooltip-content"]').first()).toHaveText('Channel 1');
});
