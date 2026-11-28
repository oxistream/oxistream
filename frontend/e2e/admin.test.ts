import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const ADMIN_ID = '0193a8e4-7f00-7000-8000-000000000001';

type MockUser = {
	id: string;
	username: string;
	role: 'admin' | 'user';
	disabled: boolean;
	auth_method: 'local' | 'oidc';
	created_at: string;
	last_seen_at: string | null;
	subscriptions: number;
};

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

function user(id: number, username: string, extra: Partial<MockUser> = {}): MockUser {
	return {
		id: `0193a8e4-7f00-7000-8000-00000000000${id}`,
		username,
		role: 'user',
		disabled: false,
		auth_method: 'local',
		created_at: `2026-11-2${id}T10:00:00Z`,
		last_seen_at: null,
		subscriptions: id,
		...extra
	};
}

/** An in-memory admin backend; `calls` records changes. */
async function mockApi(page: Page, { role = 'admin' }: { role?: 'admin' | 'user' | null } = {}) {
	const state = {
		users: [
			user(3, 'carol', { auth_method: 'oidc' }),
			user(2, 'bob', { last_seen_at: new Date().toISOString() }),
			user(1, 'mara', { role: 'admin' })
		],
		calls: [] as string[],
		/** Answer the next change with `last_admin`. */
		rejectNext: false
	};

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
			},
			sponsorblock: false,
			history_max_retention: 'forever'
		})
	);
	await page.route('**/api/v0/auth/me', (route) =>
		role
			? json(route, {
					id: ADMIN_ID,
					username: 'mara',
					role,
					has_password: true,
					created_at: '2026-11-21T10:00:00Z'
				})
			: json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/subscriptions', (route) => json(route, []));
	await page.route('**/api/v0/admin/users?*', (route) => {
		const q = new URL(route.request().url()).searchParams.get('q')?.toLowerCase() ?? '';
		const users = state.users.filter((u) => u.username.includes(q));
		return json(route, { users, total: users.length });
	});
	await page.route('**/api/v0/admin/users/*', (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		const target = state.users.find((u) => u.id === id);
		if (!target) return json(route, { code: 'not_found', message: '' }, 404);
		if (state.rejectNext) {
			state.rejectNext = false;
			return json(route, { code: 'last_admin', message: '' }, 409);
		}
		if (request.method() === 'PATCH') {
			const change = request.postDataJSON() as Partial<Pick<MockUser, 'role' | 'disabled'>>;
			state.calls.push(`PATCH ${target.username} ${JSON.stringify(change)}`);
			Object.assign(target, change);
			return route.fulfill({ status: 204 });
		}
		if (request.method() === 'DELETE') {
			state.calls.push(`DELETE ${target.username}`);
			state.users = state.users.filter((u) => u !== target);
			return route.fulfill({ status: 204 });
		}
		return route.fallback();
	});
	await page.route('**/api/v0/admin/status', (route) =>
		json(route, {
			version: '0.1.0',
			git_sha: 'abcdef1234567890',
			started_at: '2026-11-25T10:00:00Z',
			uptime_seconds: 3 * 86_400 + 4 * 3600,
			database: { reachable: true, version: '18.1' },
			counts: {
				users: 3,
				admins: 1,
				disabled_users: 0,
				channels: 12,
				subscriptions: 20,
				playlists: 4,
				videos: 340
			},
			feed: {
				interval_minutes: 30,
				running: false,
				pending: 1,
				failing: 2,
				due: 3,
				last_checked_at: '2026-11-28T09:00:00Z',
				last_run: null,
				last_error_at: null
			},
			player_cache_entries: 7,
			botguard_version: null,
			auth: {
				local_enabled: true,
				registration_enabled: true,
				require_login: false,
				oidc_enabled: true,
				oidc_discovered: false
			},
			sponsorblock: true,
			trending_region: 'US',
			rate_limit: true
		})
	);
	return state;
}

const adminLink = (page: Page) =>
	page.getByRole('navigation', { name: 'Navigation' }).getByRole('link', { name: 'Admin' });

test('guests and users get no admin pages', async ({ page }) => {
	await mockApi(page, { role: null });
	await page.goto('/');
	await expect(page.getByRole('link', { name: 'Settings' })).toBeVisible();
	await expect(adminLink(page)).toHaveCount(0);

	await mockApi(page, { role: 'user' });
	await page.goto('/admin');
	await expect(page.getByRole('heading', { name: 'Admins only' })).toBeVisible();
	await expect(adminLink(page)).toHaveCount(0);
	await expect(page.getByTestId('admin-users')).toHaveCount(0);
});

test('admins manage users from the table', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/');
	await adminLink(page).click();
	await expect(page).toHaveURL(/\/admin$/);

	const rows = page.getByTestId('admin-user-row');
	await expect(rows).toHaveCount(3);
	await expect(page.getByTestId('admin-user-count')).toHaveText('3 users');
	const row = (name: string) =>
		page.locator(`[data-testid="admin-user-row"][data-username="${name}"]`);
	await expect(row('mara').getByTestId('admin-user-menu')).toHaveCount(0);
	await expect(row('mara').getByText('You')).toBeVisible();

	await page.getByRole('button', { name: 'Actions for bob' }).click();
	await page.getByRole('menuitem', { name: 'Make admin' }).click();
	await expect(row('bob').getByTestId('admin-role')).toHaveText('Admin');
	await expect(page.getByText('bob is now an admin')).toBeVisible();

	await page.getByRole('button', { name: 'Actions for carol' }).click();
	await page.getByRole('menuitem', { name: 'Disable' }).click();
	const dialog = page.getByRole('alertdialog');
	await expect(dialog.getByRole('heading', { name: 'Disable carol?' })).toBeVisible();
	await dialog.getByRole('button', { name: 'Disable' }).click();
	await expect(row('carol').getByTestId('admin-disabled')).toBeVisible();

	state.rejectNext = true;
	await page.getByRole('button', { name: 'Actions for bob' }).click();
	await page.getByRole('menuitem', { name: 'Remove admin' }).click();
	await expect(page.getByText('At least one active admin is required')).toBeVisible();
	await expect(row('bob').getByTestId('admin-role')).toHaveText('Admin');

	await page.getByRole('button', { name: 'Actions for carol' }).click();
	await page.getByRole('menuitem', { name: 'Delete' }).click();
	await expect(dialog.getByRole('heading', { name: 'Delete carol?' })).toBeVisible();
	await dialog.getByRole('button', { name: 'Delete' }).click();
	await expect(rows).toHaveCount(2);
	await expect(page.getByText('carol deleted')).toBeVisible();

	await page.getByTestId('admin-search').fill('bo');
	await expect(rows).toHaveCount(1);
	await expect(rows.first()).toHaveAttribute('data-username', 'bob');

	expect(state.calls).toEqual([
		'PATCH bob {"role":"admin"}',
		'PATCH carol {"disabled":true}',
		'DELETE carol'
	]);
});

test('the status page shows instance health', async ({ page }) => {
	await mockApi(page);
	await page.goto('/admin/status');
	await expect(page.getByRole('link', { name: 'Status' })).toHaveAttribute('aria-current', 'page');

	const server = page.getByTestId('admin-status-server');
	await expect(server.getByText('abcdef123456')).toBeVisible();
	await expect(server.getByText('3d 4h')).toBeVisible();
	await expect(server.getByText('Not found')).toBeVisible();
	await expect(page.getByTestId('admin-status-database').getByText('18.1')).toBeVisible();
	const feed = page.getByTestId('admin-status-feed');
	await expect(feed.getByText('30 min')).toBeVisible();
	await expect(feed.getByText('Idle')).toBeVisible();
	await expect(page.getByTestId('admin-status-data').getByText('340')).toBeVisible();
	await expect(page.getByTestId('admin-status-config').getByText('Discovering')).toBeVisible();
});
