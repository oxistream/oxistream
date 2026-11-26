import { expect, test } from '@playwright/test';

test.use({ locale: 'en-US' });

test('app shell loads and the language can be switched', async ({ page }) => {
	await page.goto('/');

	const sidebar = page.locator('[data-slot="sidebar"]');
	await expect(sidebar).toBeVisible();
	await expect(sidebar.getByRole('link', { name: 'Subscriptions' })).toBeVisible();
	await expect(page.getByRole('heading', { level: 1, name: 'Home' })).toBeVisible();

	await page.getByTestId('language-switch').click();
	await page.getByRole('menuitemradio', { name: 'Deutsch' }).click();

	await expect(page.getByRole('heading', { level: 1, name: 'Startseite' })).toBeVisible();
	await expect(sidebar.getByRole('link', { name: 'Abos' })).toBeVisible();
	await expect(page.locator('html')).toHaveAttribute('lang', 'de');
});

test('deep links are served by the SPA fallback', async ({ page }) => {
	await page.goto('/feed/history');
	await expect(page.getByRole('heading', { level: 1, name: 'History' })).toBeVisible();
});
