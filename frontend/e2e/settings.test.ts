import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	has_password: true,
	created_at: '2026-11-26T10:00:00Z'
};

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

type Calls = { patches: unknown[]; password: unknown[]; deletions: unknown[]; signedOut: boolean };

async function mockApi(
	page: Page,
	{ signedIn = true, hasPassword = true }: { signedIn?: boolean; hasPassword?: boolean } = {}
): Promise<Calls> {
	const calls: Calls = { patches: [], password: [], deletions: [], signedOut: false };
	let settings: Record<string, unknown> = {
		video_codecs: [],
		max_quality: null,
		default_quality: null,
		home_sections: null
	};
	let paused = false;
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
		signedIn && !calls.signedOut
			? json(route, { ...account, has_password: hasPassword })
			: json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/auth/logout', (route) => {
		calls.signedOut = true;
		return json(route, { end_session_url: null });
	});
	await page.route('**/api/v0/settings', (route) => {
		if (route.request().method() === 'PATCH') {
			const patch = route.request().postDataJSON() as Record<string, unknown>;
			calls.patches.push(patch);
			settings = { ...settings, ...patch };
		}
		return json(route, settings);
	});
	await page.route('**/api/v0/history/settings', (route) => {
		if (route.request().method() === 'PUT') {
			paused = (route.request().postDataJSON() as { paused: boolean }).paused;
		}
		return json(route, { paused });
	});
	await page.route('**/api/v0/account/password', (route) => {
		const body = route.request().postDataJSON() as { current_password: string };
		calls.password.push(body);
		return body.current_password === 'old password!!'
			? route.fulfill({ status: 204 })
			: json(
					route,
					{
						code: 'validation_failed',
						message: '',
						fields: { current_password: 'mismatch' }
					},
					422
				);
	});
	await page.route('**/api/v0/account', (route) => {
		calls.deletions.push(route.request().postDataJSON());
		return route.fulfill({ status: 204 });
	});
	await page.route('**/api/v0/trending?*', (route) =>
		json(route, { category: 'news', region: 'US', items: [] })
	);
	return calls;
}

test('guests only get appearance settings and a sign-in prompt', async ({ page }) => {
	await mockApi(page, { signedIn: false });
	await page.goto('/settings');
	await expect(page.getByRole('heading', { name: 'Sign in for more settings' })).toBeVisible();
	await expect(page.getByRole('heading', { level: 2 })).toHaveText([
		'Sign in for more settings',
		'Appearance'
	]);
});

test('home page sections can be reordered, hidden and reset', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings');

	const shown = page.getByRole('list', { name: 'Shown' });
	await expect(shown.getByRole('listitem')).toHaveText([
		/Continue watching/,
		/Latest from your subscriptions/,
		/Shorts from your subscriptions/
	]);
	await shown.getByRole('button', { name: 'Move Latest from your subscriptions up' }).click();
	await expect
		.poll(() => calls.patches.at(-1))
		.toEqual({
			home_sections: ['subscriptions', 'continue-watching', 'subscription-shorts']
		});

	await page.getByRole('switch', { name: 'Show Trending: News' }).click();
	await expect(shown.getByRole('listitem')).toHaveCount(4);
	await expect
		.poll(() => calls.patches.at(-1))
		.toEqual({
			home_sections: ['subscriptions', 'continue-watching', 'subscription-shorts', 'trending-news']
		});

	// Keyboard alternative to dragging.
	await shown.getByRole('button', { name: 'Reorder Trending: News' }).press('Home');
	await expect(shown.getByRole('listitem').first()).toHaveText(/Trending: News/);

	await shown.getByRole('switch', { name: 'Show Shorts from your subscriptions' }).click();
	await expect(shown.getByRole('listitem')).toHaveCount(3);

	await page.getByRole('button', { name: 'Reset to default' }).click();
	await expect.poll(() => calls.patches.at(-1)).toEqual({ home_sections: null });
	await expect(shown.getByRole('listitem')).toHaveCount(3);
	await expect(page.getByRole('button', { name: 'Reset to default' })).toBeDisabled();
});

test('playback preferences are saved', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings');

	await page.getByText('Custom order', { exact: true }).click();
	await expect.poll(() => calls.patches.at(-1)).toEqual({ video_codecs: ['av1', 'vp9', 'h264'] });
	const order = page.getByRole('list', { name: 'Codec order' });
	await order.getByRole('button', { name: 'Move H.264 up' }).click();
	await expect.poll(() => calls.patches.at(-1)).toEqual({ video_codecs: ['av1', 'h264', 'vp9'] });

	await page.getByRole('combobox', { name: 'Maximum quality' }).click();
	await page.getByRole('option', { name: '1080p (HD)' }).click();
	await expect.poll(() => calls.patches.at(-1)).toEqual({ max_quality: 1080 });
	await expect(page.getByRole('combobox', { name: 'Maximum quality' })).toHaveText(/1080p/);

	await page.getByText('Automatic', { exact: true }).first().click();
	await expect.poll(() => calls.patches.at(-1)).toEqual({ video_codecs: [] });
	await expect(order).toHaveCount(0);
});

test('blocked channels are managed on their own page', async ({ page }) => {
	await mockApi(page);
	await page.goto('/settings');
	const blocked = page.getByRole('region', { name: 'Blocked channels' });
	await expect(blocked.getByRole('link', { name: 'Manage' })).toHaveAttribute(
		'href',
		'/settings/blocked'
	);
});

test('history can be paused from the settings', async ({ page }) => {
	await mockApi(page);
	await page.goto('/settings');
	const toggle = page.getByRole('switch', { name: 'Pause history' });
	await expect(toggle).not.toBeChecked();
	await toggle.click();
	await expect(toggle).toBeChecked();
});

test('the password can be changed', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings');

	await page.getByLabel('Current password').fill('wrong password');
	await page.getByLabel('New password').fill('new password!!');
	await page.getByLabel('Confirm password').fill('new password!!');
	await page.getByRole('button', { name: 'Change password' }).click();
	await expect(page.getByText('The password is incorrect.')).toBeVisible();

	await page.getByLabel('Current password').fill('old password!!');
	await page.getByRole('button', { name: 'Change password' }).click();
	await expect(page.getByTestId('password-status')).toHaveText(/Password changed/);
	expect(calls.password).toHaveLength(2);
	await expect(page.getByLabel('Current password')).toHaveValue('');
});

test('single sign-on users have no password form', async ({ page }) => {
	await mockApi(page, { hasPassword: false });
	await page.goto('/settings');
	await expect(page.getByText(/Single sign-on account since/)).toBeVisible();
	await expect(page.getByLabel('Current password')).toHaveCount(0);
});

test('deleting the account needs the username and password', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings');

	await page.getByRole('button', { name: 'Delete account' }).click();
	const dialog = page.getByRole('alertdialog');
	const confirm = dialog.getByRole('button', { name: 'Delete account' });
	await expect(confirm).toBeDisabled();
	await dialog.getByLabel('Type mara to confirm').fill('mar');
	await dialog.getByLabel('Password').fill('old password!!');
	await expect(confirm).toBeDisabled();
	await dialog.getByLabel('Type mara to confirm').fill('Mara');
	await confirm.click();

	await expect(page).toHaveURL(/\/$/);
	expect(calls.deletions).toEqual([{ username: 'Mara', password: 'old password!!' }]);
	await expect(page.getByRole('button', { name: /Not signed in/ })).toBeVisible();
});
