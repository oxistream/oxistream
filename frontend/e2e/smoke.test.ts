import { expect, test } from '@playwright/test';

test.use({ locale: 'en-US' });

test('app shell loads and the language can be switched', async ({ page }) => {
	await page.goto('/');

	const sidebar = page.locator('[data-slot="sidebar"]');
	await expect(sidebar).toBeVisible();
	await expect(sidebar.getByRole('link', { name: 'Subscriptions' })).toBeVisible();
	await expect(page.getByRole('heading', { level: 1, name: 'Home' })).toBeAttached();
	await expect(page.getByRole('heading', { level: 2, name: 'Continue watching' })).toBeVisible();
	await expect(page).toHaveTitle('Oxistream');

	await page.getByTestId('language-switch').click();
	await page.getByRole('menuitemradio', { name: 'Deutsch' }).click();

	await expect(page.getByRole('heading', { level: 1, name: 'Startseite' })).toBeAttached();
	await expect(sidebar.getByRole('link', { name: 'Abos' })).toBeVisible();
	await expect(page.locator('html')).toHaveAttribute('lang', 'de');
});

test('deep links are served by the SPA fallback', async ({ page }) => {
	await page.goto('/feed/history');
	await expect(page.getByRole('heading', { level: 1, name: 'History' })).toBeVisible();
	await expect(page).toHaveTitle('History · Oxistream');
});

test('slash focuses the search field', async ({ page }) => {
	await page.goto('/');
	await page.locator('body').press('/');
	await expect(page.getByTestId('search-input')).toBeFocused();
});

test('theme can be switched', async ({ page }) => {
	await page.emulateMedia({ colorScheme: 'light' });
	await page.goto('/');
	const html = page.locator('html');
	await expect(html).not.toHaveClass(/dark/);

	await page.getByTestId('theme-switch').click();
	await page.getByRole('menuitemradio', { name: 'Dark' }).click();
	await expect(html).toHaveClass(/dark/);

	await page.reload();
	await expect(html).toHaveClass(/dark/);
});

test('login page renders and validates', async ({ page }) => {
	await page.goto('/login');
	await expect(page).toHaveTitle('Sign in · Oxistream');
	await expect(page.getByRole('heading', { level: 1, name: 'Sign in to Oxistream' })).toBeVisible();
	await expect(page.getByTestId('sso-button')).toBeVisible();

	await page.getByRole('button', { name: 'Sign in', exact: true }).click();
	const username = page.getByLabel('Username');
	await expect(username).toHaveAttribute('aria-invalid', 'true');
	await expect(username).toBeFocused();
	await expect(page.getByTestId('form-status')).toHaveText('2 fields need attention.');

	await page.getByRole('link', { name: 'Create one' }).click();
	await expect(page).toHaveURL(/\/register$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Create your account' })).toBeVisible();
});

test.describe('on a phone', () => {
	test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

	test('the sidebar opens as a sheet and closes after navigating', async ({ page }) => {
		await page.goto('/');
		const trigger = page.getByRole('button', { name: 'Toggle sidebar' });
		await expect(trigger).toHaveAttribute('aria-expanded', 'false');

		await trigger.click();
		const sheet = page.getByRole('dialog');
		await expect(sheet).toBeVisible();
		await sheet.getByRole('link', { name: 'Playlists' }).click();

		await expect(page).toHaveURL(/\/feed\/playlists$/);
		await expect(sheet).toBeHidden();
		await expect(page.getByRole('heading', { level: 1, name: 'Playlists' })).toBeVisible();
	});
});
