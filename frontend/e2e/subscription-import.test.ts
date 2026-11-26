import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

const json = (route: Route, status: number, body: unknown) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

async function mockApi(page: Page, signedIn: boolean) {
	await page.route('**/api/v0/**', (route) => json(route, 404, { code: 'not_found', message: '' }));
	await page.route('**/api/v0/instance', (route) =>
		json(route, 200, {
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
		signedIn
			? json(route, 200, account)
			: json(route, 401, { code: 'unauthorized', message: 'sign in required' })
	);
}

const csv = 'Channel Id,Channel Url,Channel Title\nUCsXVk37bltHxD1rDPwtNM8Q,,Kurzgesagt\n';

test('imports a file and shows the counts', async ({ page }) => {
	await mockApi(page, true);
	let sent: { body: string | null; type: string | undefined } | undefined;
	await page.route('**/api/v0/subscriptions/import', async (route) => {
		const request = route.request();
		sent = { body: request.postData(), type: request.headers()['content-type'] };
		await json(route, 200, {
			format: 'youtube_takeout',
			imported: 12,
			already_subscribed: 3,
			skipped: 1,
			invalid: 0,
			skipped_entries: ['https://www.youtube.com/@handle']
		});
	});

	await page.goto('/subscriptions/import-export');
	await page.getByLabel('Subscription file').setInputFiles({
		name: 'subscriptions.csv',
		mimeType: 'text/csv',
		buffer: Buffer.from(csv)
	});

	await expect(page.getByTestId('import-format')).toHaveText('YouTube Takeout');
	expect(sent).toEqual({ body: csv, type: 'application/octet-stream' });
	await expect(page.getByText('Imported', { exact: true })).toBeVisible();
	await expect(page.locator('dd').first()).toHaveText('12');
	await page.getByText('Show skipped entries (1)').click();
	await expect(page.getByText('https://www.youtube.com/@handle')).toBeVisible();

	await expect(page.getByRole('link', { name: 'Download: NewPipe JSON' })).toHaveAttribute(
		'href',
		'/api/v0/subscriptions/export?format=newpipe'
	);
});

test('explains rejected files', async ({ page }) => {
	await mockApi(page, true);
	await page.route('**/api/v0/subscriptions/import', (route) =>
		json(route, 400, { code: 'bad_request', message: 'unrecognized file format' })
	);
	await page.goto('/subscriptions/import-export');
	await page.getByLabel('Subscription file').setInputFiles({
		name: 'notes.txt',
		mimeType: 'text/plain',
		buffer: Buffer.from('hello')
	});
	await expect(page.getByRole('alert')).toContainText('not a supported subscription export');
});

test('asks guests to sign in', async ({ page }) => {
	await mockApi(page, false);
	await page.goto('/subscriptions/import-export');
	await expect(page.getByText('Sign in to import subscriptions')).toBeVisible();
	await expect(page.getByLabel('Subscription file')).toHaveCount(0);
});
