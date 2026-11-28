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

const channel = {
	id: 'UCm0ck4nv1lF0rg3H4mm3r1',
	name: 'Hammer & Tongs',
	avatar: [],
	verified: true
};

function entry(n: number, watchedAt: Date, position = 60) {
	const id = `hist${String(n).padStart(6, '0')}A`;
	return {
		video: {
			id,
			title: `Watched video ${n}`,
			thumbnails: [{ url: `/proxy/image/h${n}`, width: 320, height: 180 }],
			duration_seconds: 200,
			published_at: null,
			published_text: null,
			view_count: null,
			is_live: false,
			is_short: false,
			is_upcoming: false,
			channel
		},
		progress: {
			video_id: id,
			position_seconds: position,
			duration_seconds: 200,
			finished: false,
			updated_at: watchedAt.toISOString()
		}
	};
}

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

async function mockApi(page: Page, { signedIn = true } = {}) {
	const now = Date.now();
	const state = {
		items: [
			entry(1, new Date(now - 60_000)),
			entry(2, new Date(now - 120_000), 100),
			entry(3, new Date(now - 3 * 24 * 3600_000))
		],
		paused: false,
		retention: 'forever',
		calls: [] as string[],
		searches: [] as string[]
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
		signedIn ? json(route, account) : json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/subscriptions', (route) => json(route, []));
	await page.route('**/api/v0/history?*', (route) => {
		const q = new URL(route.request().url()).searchParams.get('q') ?? '';
		state.searches.push(q);
		const items = state.items.filter(
			(i) =>
				i.video.title.toLowerCase().includes(q.toLowerCase()) ||
				i.video.channel.name.toLowerCase().includes(q.toLowerCase())
		);
		return json(route, { items, continuation: null });
	});
	await page.route('**/api/v0/history', (route) => {
		state.calls.push(`${route.request().method()} all`);
		state.items = [];
		return route.fulfill({ status: 204 });
	});
	await page.route('**/api/v0/history/continue?*', (route) => json(route, state.items.slice(0, 2)));
	await page.route('**/api/v0/history/settings', (route) => {
		if (route.request().method() === 'PUT') {
			const body = route.request().postDataJSON() as { paused?: boolean };
			if (body.paused !== undefined) state.paused = body.paused;
			state.calls.push(`PUT paused=${state.paused}`);
		}
		return json(route, { paused: state.paused, retention: state.retention });
	});
	await page.route(/\/api\/v0\/history\/hist\w+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		state.calls.push(`${route.request().method()} ${id}`);
		state.items = state.items.filter((i) => i.video.id !== id);
		return route.fulfill({ status: 204 });
	});
	return state;
}

test('anonymous users get a sign-in prompt on the history page', async ({ page }) => {
	await mockApi(page, { signedIn: false });
	await page.goto('/feed/history');
	await expect(page.getByRole('heading', { level: 1, name: 'History' })).toBeVisible();
	await expect(page.getByRole('main').getByRole('link', { name: 'Sign in' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Clear history' })).toHaveCount(0);
});

test('history lists watched videos by day and removes them', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/history');

	const today = page.getByRole('list', { name: 'Today' });
	await expect(today.getByRole('listitem')).toHaveCount(2);
	await expect(today.getByRole('link', { name: /Watched video 1.*30% watched/ })).toHaveAttribute(
		'href',
		'/watch?v=hist000001A'
	);
	await expect(page.getByRole('heading', { level: 2 })).toHaveCount(2);

	await page.getByRole('button', { name: 'Remove “Watched video 1” from history' }).click();
	await expect(today.getByRole('listitem')).toHaveCount(1);
	expect(state.calls).toContain('DELETE hist000001A');
});

test('clearing the history asks for confirmation', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/history');
	await expect(page.getByRole('link', { name: /Watched video 3/ })).toBeVisible();

	await page.getByRole('button', { name: 'Clear history' }).click();
	const dialog = page.getByRole('alertdialog', { name: 'Clear your watch history?' });
	await dialog.getByRole('button', { name: 'Cancel' }).click();
	await expect(dialog).toHaveCount(0);
	expect(state.calls).toEqual([]);

	await page.getByRole('button', { name: 'Clear history' }).click();
	await page.getByRole('alertdialog').getByRole('button', { name: 'Clear history' }).click();
	await expect(page.getByRole('heading', { name: 'Your history is empty' })).toBeVisible();
	expect(state.calls).toEqual(['DELETE all']);
});

test('history can be paused and resumed', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/history');

	await page.getByRole('button', { name: 'Pause history' }).click();
	await expect(page.getByText('History is paused')).toBeVisible();
	await page.getByRole('button', { name: 'Resume history' }).click();
	await expect(page.getByText('History is paused')).toHaveCount(0);
	expect(state.calls).toEqual(['PUT paused=true', 'PUT paused=false']);
});

test('home shows continue watching for signed-in users only', async ({ page }) => {
	await mockApi(page);
	await page.goto('/');
	const row = page.getByRole('list', { name: 'Continue watching' });
	await expect(row.getByRole('listitem')).toHaveCount(2);
	await expect(row.getByRole('link', { name: /Watched video 2.*50% watched/ })).toBeVisible();
	await page
		.getByRole('region', { name: 'Continue watching' })
		.getByRole('link', { name: 'Show all' })
		.click();
	await expect(page).toHaveURL(/\/feed\/history$/);
});

test('anonymous users see no continue watching row', async ({ page }) => {
	await mockApi(page, { signedIn: false });
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'What do you want to watch?' })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Continue watching' })).toHaveCount(0);
});

test('feed cards show the progress of started videos', async ({ page }) => {
	await mockApi(page);
	const feedVideo = (n: number) => ({
		...entry(n, new Date()).video,
		id: `feed${String(n).padStart(6, '0')}A`,
		title: `Feed video ${n}`
	});
	await page.route('**/api/v0/subscriptions', (route) =>
		json(route, [
			{
				...channel,
				handle: null,
				subscriber_count: null,
				subscribed_at: '2026-11-20T10:00:00Z',
				avatar: []
			}
		])
	);
	await page.route('**/api/v0/feed?*', (route) =>
		json(route, { items: [1, 2].map(feedVideo), continuation: null })
	);
	let requested = '';
	await page.route('**/api/v0/progress?*', (route) => {
		requested = new URL(route.request().url()).searchParams.get('ids') ?? '';
		return json(route, [
			{
				video_id: 'feed000002A',
				position_seconds: 150,
				duration_seconds: 200,
				finished: false,
				updated_at: '2026-11-26T10:00:00Z'
			}
		]);
	});
	await page.goto('/feed/subscriptions');
	const feed = page.getByRole('list', { name: 'Subscriptions' });
	await expect(feed.getByRole('link', { name: /Feed video 2.*75% watched/ })).toBeVisible();
	await expect(feed.getByRole('link', { name: /Feed video 1/ })).not.toHaveAccessibleName(
		/watched/
	);
	expect(requested.split(',').sort()).toEqual(['feed000001A', 'feed000002A']);
});

test('the watch page resumes from the server unless a start time is given', async ({ page }) => {
	await mockApi(page);
	const requests: string[] = [];
	await page.route('**/api/v0/progress/*', (route) => {
		requests.push(new URL(route.request().url()).pathname);
		return json(route, { progress: null });
	});
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) => json(route, { code: 'not_found' }, 404));

	await page.goto('/watch?v=k3Rv9fTq2Lw');
	await expect.poll(() => requests).toEqual(['/api/v0/progress/k3Rv9fTq2Lw']);

	await page.goto('/watch?v=dQw4w9WgXcQ&t=42');
	await page.waitForLoadState('networkidle');
	expect(requests).toEqual(['/api/v0/progress/k3Rv9fTq2Lw']);
});

test('history search filters on the server and survives a reload', async ({ page }) => {
	const state = await mockApi(page);
	state.items[1].video.title = 'Baking bread';
	await page.goto('/feed/history');
	await expect(page.getByRole('link', { name: /Watched video 3/ })).toBeVisible();

	await page.getByRole('searchbox', { name: 'Search history' }).fill('bread');
	await expect(page).toHaveURL(/\/feed\/history\?q=bread$/);
	await expect(page.getByRole('link', { name: /Baking bread/ })).toBeVisible();
	await expect(page.getByRole('link', { name: /Watched video/ })).toHaveCount(0);
	expect(state.searches.filter((q) => q === 'bread')).toHaveLength(1);

	await page.reload();
	await expect(page.getByRole('searchbox', { name: 'Search history' })).toHaveValue('bread');
	await expect(page.getByRole('link', { name: /Baking bread/ })).toBeVisible();

	await page.getByRole('searchbox', { name: 'Search history' }).fill('nothing here');
	await expect(page.getByRole('heading', { name: 'No results for “nothing here”' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Clear history' })).toBeEnabled();

	await page.getByRole('searchbox', { name: 'Search history' }).fill('');
	await expect(page).toHaveURL(/\/feed\/history$/);
	await expect(page.getByRole('link', { name: /Watched video 3/ })).toBeVisible();
});
