import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

type Capabilities = {
	local_enabled: boolean;
	registration_enabled: boolean;
	require_login: boolean;
	oidc_enabled: boolean;
};

const defaults: Capabilities = {
	local_enabled: true,
	registration_enabled: true,
	require_login: false,
	oidc_enabled: false
};

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'admin',
	created_at: '2026-11-26T10:00:00Z'
};

const json = (route: Route, status: number, body?: unknown) =>
	route.fulfill({
		status,
		contentType: 'application/json',
		body: body === undefined ? '' : JSON.stringify(body)
	});

const unauthorized = { code: 'unauthorized', message: 'sign in required' };

/** Mocks the instance and session endpoints; `signedIn` is mutable so actions can change it. */
async function mockApi(page: Page, capabilities: Partial<Capabilities> = {}) {
	const state = { signedIn: false };
	await page.route('**/api/v0/**', (route) => json(route, 404, { code: 'not_found', message: '' }));
	await page.route('**/api/v0/instance', (route) =>
		json(route, 200, {
			name: 'Oxistream',
			version: '0.1.0',
			auth: { ...defaults, ...capabilities }
		})
	);
	await page.route('**/api/v0/auth/me', (route) =>
		state.signedIn ? json(route, 200, account) : json(route, 401, unauthorized)
	);
	return state;
}

test('signing in continues to the redirect target', async ({ page }) => {
	const state = await mockApi(page);
	let sent: unknown;
	await page.route('**/api/v0/auth/login', async (route) => {
		sent = route.request().postDataJSON();
		state.signedIn = true;
		await json(route, 200, account);
	});

	await page.goto('/login?redirect=%2Ffeed%2Fhistory');
	await page.getByLabel('Username').fill(' mara ');
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign in', exact: true }).click();

	await expect(page).toHaveURL(/\/feed\/history$/);
	expect(sent).toEqual({ username: 'mara', password: 'correct horse battery' });
	await expect(page.getByTestId('user-menu')).toContainText('@mara');
});

test('a failed sign-in shows a generic error', async ({ page }) => {
	await mockApi(page);
	await page.route('**/api/v0/auth/login', (route) =>
		json(route, 401, { code: 'invalid_credentials', message: 'invalid username or password' })
	);

	await page.goto('/login');
	await page.getByLabel('Username').fill('mara');
	await page.getByLabel('Password').fill('wrong password');
	await page.getByRole('button', { name: 'Sign in', exact: true }).click();

	await expect(page.getByTestId('form-status')).toHaveText('Username or password is incorrect.');
	await expect(page.getByLabel('Password')).toHaveValue('');
	await expect(page.getByLabel('Password')).toBeFocused();
	await expect(page).toHaveURL(/\/login$/);
});

test('registration shows server field errors, then signs in', async ({ page }) => {
	const state = await mockApi(page);
	let attempts = 0;
	await page.route('**/api/v0/auth/register', async (route) => {
		attempts += 1;
		if (attempts === 1) {
			await json(route, 409, {
				code: 'validation_failed',
				message: 'invalid input',
				fields: { username: 'taken' }
			});
			return;
		}
		state.signedIn = true;
		await json(route, 201, account);
	});

	await page.goto('/register');
	await page.getByLabel('Username').fill('mara');
	await page.getByLabel('Password', { exact: true }).fill('correct horse battery');
	await page.getByLabel('Confirm password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Create account' }).click();

	const username = page.getByLabel('Username');
	await expect(username).toHaveAttribute('aria-invalid', 'true');
	await expect(page.getByText('This username is already taken.')).toBeVisible();
	await expect(username).toBeFocused();

	await username.fill('mara2');
	await page.getByRole('button', { name: 'Create account' }).click();
	await expect(page).toHaveURL(/\/$/);
	await expect(page.getByTestId('user-menu')).toContainText('@mara');
});

test('signing out returns to the guest menu', async ({ page }) => {
	const state = await mockApi(page);
	state.signedIn = true;
	let loggedOut = false;
	await page.route('**/api/v0/auth/logout', async (route) => {
		loggedOut = true;
		state.signedIn = false;
		await route.fulfill({ status: 204 });
	});

	await page.goto('/');
	const menu = page.getByTestId('user-menu');
	await expect(menu).toContainText('@mara');
	await menu.click();
	await page.getByRole('menuitem', { name: 'Sign out' }).click();

	await expect(menu).toContainText('Not signed in');
	expect(loggedOut).toBe(true);
});

test('anonymous users are sent to sign in when login is required', async ({ page }) => {
	await mockApi(page, { require_login: true });
	await page.goto('/feed/subscriptions');
	await expect(page).toHaveURL(/\/login\?redirect=%2Ffeed%2Fsubscriptions$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Sign in to Oxistream' })).toBeVisible();
});

test('closed registration hides the sign-up form and links', async ({ page }) => {
	await mockApi(page, { registration_enabled: false });
	await page.goto('/login');
	await expect(page.getByRole('heading', { level: 1, name: 'Sign in to Oxistream' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Create one' })).toHaveCount(0);

	await page.goto('/register');
	await expect(page.getByTestId('registration-closed')).toContainText('Registration is closed');
	await expect(page.getByLabel('Confirm password')).toHaveCount(0);
});

test('without local accounts only the disabled SSO button remains', async ({ page }) => {
	await mockApi(page, { local_enabled: false, registration_enabled: false });
	await page.goto('/login');
	await expect(page.getByTestId('sso-button')).toBeDisabled();
	await expect(page.getByLabel('Username')).toHaveCount(0);
	await expect(page.getByTestId('form-status')).toHaveText(
		'This instance only offers single sign-on.'
	);
});
