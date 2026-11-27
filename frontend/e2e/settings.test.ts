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
	{
		signedIn = true,
		hasPassword = true,
		sponsorblock = false
	}: { signedIn?: boolean; hasPassword?: boolean; sponsorblock?: boolean } = {}
): Promise<Calls> {
	const calls: Calls = { patches: [], password: [], deletions: [], signedOut: false };
	let settings: Record<string, unknown> = {
		video_codecs: [],
		max_quality: null,
		default_quality: null,
		home_sections: null,
		audio_language: null
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
			},
			sponsorblock
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

test('guests get browser-only settings and a sign-in line', async ({ page }) => {
	await mockApi(page, { signedIn: false });
	await page.goto('/settings');
	await expect(page).toHaveURL(/\/settings\/playback$/);
	const sections = page.getByRole('navigation', { name: 'Settings sections' });
	await expect(sections.getByRole('listitem')).toHaveText(['Playback', 'Appearance']);
	const hint = sections.getByTestId('settings-guest-hint');
	await expect(hint).toHaveText(/Sign in for more settings/);
	await expect(hint.getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/login');

	// Account sections show the same line instead of their content.
	await page.goto('/settings/home');
	await expect(page.getByRole('main').getByTestId('settings-guest-hint')).toHaveCount(1);
	await page.goto('/settings/blocked');
	await expect(page.getByRole('main').getByTestId('settings-guest-hint')).toHaveCount(1);

	// Open on settings routes.
	const nav = page.locator('[data-slot="sidebar"]');
	await expect(nav.getByRole('list', { name: 'Settings sections' }).getByRole('link')).toHaveText([
		'Playback',
		'Appearance'
	]);

	await page.goto('/settings/playback');
	await expect(page.getByRole('combobox', { name: 'Maximum quality' })).toHaveCount(0);
	const audio = page.getByRole('combobox', { name: 'Default audio track' });
	await expect(audio).toHaveText('Original');
	await audio.click();
	await page.getByRole('option', { name: 'German' }).click();
	await expect(audio).toHaveText('German');
	expect(await page.evaluate(() => localStorage.getItem('oxistream:audio-language'))).toBe('"de"');
});

test('signed-in users store the audio track preference', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings/playback');
	const audio = page.getByRole('combobox', { name: 'Default audio track' });
	await expect(audio).toHaveText('Original');
	await audio.click();
	await page.getByRole('option', { name: 'Interface language (English)' }).click();
	await expect(audio).toHaveText('Interface language (English)');
	expect(calls.patches).toContainEqual({ audio_language: 'interface' });
});

test('sponsorblock has its own section when the server offers it', async ({ page }) => {
	await mockApi(page, { signedIn: false, sponsorblock: true });
	await page.goto('/settings/playback');
	const sections = page.getByRole('navigation', { name: 'Settings sections' });
	await expect(sections.getByRole('listitem')).toHaveText([
		'Playback',
		'SponsorBlock',
		'Appearance'
	]);

	await page.locator('[data-slot="sidebar"]').getByRole('link', { name: 'SponsorBlock' }).click();
	await expect(page).toHaveURL(/\/settings\/sponsorblock$/);
	await expect(page.getByRole('heading', { level: 1, name: 'SponsorBlock' })).toBeVisible();
	await page.locator('[data-category="sponsor"]').getByText('Off').click();
	const stored = await page.evaluate(() => localStorage.getItem('oxistream:sponsorblock'));
	expect((JSON.parse(stored!) as { categories: Record<string, string> }).categories.sponsor).toBe(
		'off'
	);
});

test('desktop opens the first section next to the section navigation', async ({ page }) => {
	await mockApi(page);
	await page.goto('/settings');
	await expect(page).toHaveURL(/\/settings\/playback$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Playback' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'All settings' })).toBeHidden();
	const sections = page.getByRole('navigation', { name: 'Settings sections' });
	await expect(sections.getByRole('link')).toHaveText([
		'Playback',
		'Home page',
		'History',
		'Blocked channels',
		'Appearance',
		'Account'
	]);
	await expect(sections.getByRole('link', { name: 'Playback' })).toHaveAttribute(
		'aria-current',
		'page'
	);
	await sections.getByRole('link', { name: 'Home page' }).click();
	await expect(page).toHaveURL(/\/settings\/home$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Home page' })).toBeVisible();
	await sections.getByRole('link', { name: 'Blocked channels' }).click();
	await expect(page.getByRole('heading', { level: 1, name: 'Blocked channels' })).toBeVisible();

	// Back leaves settings instead of returning to the redirect.
	await page.goto('/');
	await page.goto('/settings');
	await expect(page).toHaveURL(/\/settings\/playback$/);
	await page.goBack();
	await expect(page).toHaveURL(/\/$/);

	await page.goto('/settings/nope');
	await expect(page.getByRole('heading', { level: 1, name: 'Page not found' })).toBeVisible();
});

test('phones list the sections, then open one with a way back', async ({ page }) => {
	await page.setViewportSize({ width: 375, height: 812 });
	await mockApi(page);
	await page.goto('/settings');
	await expect(page).toHaveURL(/\/settings$/);
	await expect(page.getByRole('navigation', { name: 'Settings sections' })).toBeHidden();
	const overview = page.getByRole('main').getByRole('list', { name: 'Settings sections' });
	await expect(overview.getByRole('link')).toHaveText([
		/Playback/,
		/Home page/,
		/History/,
		/Blocked channels/,
		/Appearance/,
		/Account/
	]);
	await overview.getByRole('link', { name: /Blocked channels/ }).click();
	await expect(page).toHaveURL(/\/settings\/blocked$/);
	await page.getByRole('link', { name: 'All settings' }).click();
	await expect(page).toHaveURL(/\/settings$/);
});

test('guests on phones see the sections and the sign-in line', async ({ page }) => {
	await page.setViewportSize({ width: 375, height: 812 });
	await mockApi(page, { signedIn: false });
	await page.goto('/settings');
	const overview = page.getByRole('main').getByRole('list', { name: 'Settings sections' });
	await expect(overview.getByRole('link')).toHaveText([/Playback/, /Appearance/]);
	await expect(page.getByTestId('settings-guest-hint').filter({ visible: true })).toHaveCount(1);
});

test('the sidebar settings entry expands to the sections', async ({ page }) => {
	await mockApi(page);
	await page.goto('/');
	const nav = page.locator('[data-slot="sidebar"]');
	const sub = nav.getByRole('list', { name: 'Settings sections' });
	await expect(sub).toBeHidden();

	await nav.getByRole('button', { name: 'Show settings sections' }).click();
	await expect(sub.getByRole('link')).toHaveText([
		'Playback',
		'Home page',
		'History',
		'Blocked channels',
		'Appearance',
		'Account'
	]);
	await sub.getByRole('link', { name: 'Playback' }).click();
	await expect(page).toHaveURL(/\/settings\/playback$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Playback' })).toBeVisible();
	await expect(sub.getByRole('link', { name: 'Playback' })).toHaveAttribute('data-active', 'true');
	await expect(sub.getByRole('link', { name: 'Playback' })).toHaveAttribute('aria-current', 'page');
	await expect(
		page
			.getByRole('navigation', { name: 'Settings sections' })
			.getByRole('link', { name: 'Playback' })
	).toHaveAttribute('aria-current', 'page');

	await sub.getByRole('link', { name: 'Blocked channels' }).click();
	await expect(page).toHaveURL(/\/settings\/blocked$/);
	await expect(sub.getByRole('link', { name: 'Blocked channels' })).toHaveAttribute(
		'data-active',
		'true'
	);
	await expect(sub.getByRole('link', { name: 'Playback' })).toHaveAttribute('data-active', 'false');

	// Collapsed elsewhere unless left open, and remembered across reloads.
	await nav.getByRole('button', { name: 'Show settings sections' }).click();
	await expect(sub).toBeHidden();
	await nav.getByRole('link', { name: 'Home', exact: true }).click();
	await expect(sub).toBeHidden();
	await nav.getByRole('button', { name: 'Show settings sections' }).click();
	await page.reload();
	await expect(sub.getByRole('link', { name: 'Account' })).toBeVisible();
});

test('the collapsed sidebar lists the sections in the settings tooltip', async ({ page }) => {
	await mockApi(page);
	await page.goto('/');
	await page.getByRole('button', { name: 'Toggle sidebar' }).click();
	await expect(page.locator('[data-slot="sidebar"][data-state="collapsed"]')).toBeVisible();

	const gear = page.locator('[data-slot="sidebar"]').getByRole('link', { name: 'Settings' });
	await gear.hover();
	await expect(page.locator('[data-slot="tooltip-content"]').getByRole('listitem')).toHaveText([
		'Playback',
		'Home page',
		'History',
		'Blocked channels',
		'Appearance',
		'Account'
	]);

	await gear.click();
	await expect(page).toHaveURL(/\/settings\/playback$/);
	await page
		.getByRole('navigation', { name: 'Settings sections' })
		.getByRole('link', { name: 'History' })
		.click();
	await expect(page).toHaveURL(/\/settings\/history$/);
	await expect(gear).toHaveAttribute('data-active', 'true');
});

test('the mobile sheet lists the settings sections', async ({ page }) => {
	await page.setViewportSize({ width: 375, height: 812 });
	await mockApi(page);
	await page.goto('/settings/account');
	await page.getByRole('button', { name: 'Toggle sidebar' }).click();
	const sheet = page.getByRole('dialog');
	const sub = sheet.getByRole('list', { name: 'Settings sections' });
	await expect(sub.getByRole('link', { name: 'Account' })).toHaveAttribute('data-active', 'true');
	await sub.getByRole('link', { name: 'History' }).click();
	await expect(page).toHaveURL(/\/settings\/history$/);
	await expect(sheet).toBeHidden();
	await page.getByRole('link', { name: 'All settings' }).click();
	await expect(page).toHaveURL(/\/settings$/);
});

test('home page sections can be reordered, hidden and reset', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings/home');

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
	await page.goto('/settings/playback');

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

test('history can be paused from the settings', async ({ page }) => {
	await mockApi(page);
	await page.goto('/settings/history');
	const toggle = page.getByRole('switch', { name: 'Pause history' });
	await expect(toggle).not.toBeChecked();
	await toggle.click();
	await expect(toggle).toBeChecked();
});

test('the password can be changed', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings/account');

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
	await page.goto('/settings/account');
	await expect(page.getByText(/Single sign-on account since/)).toBeVisible();
	await expect(page.getByLabel('Current password')).toHaveCount(0);
});

test('deleting the account needs the username and password', async ({ page }) => {
	const calls = await mockApi(page);
	await page.goto('/settings/account');

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
