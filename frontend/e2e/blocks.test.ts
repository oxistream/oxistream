import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

const FORGE = { id: 'UCforgeforgeforgeforge01', name: 'Hammer & Tongs' };
const NORTH = { id: 'UCnorthnorthnorthnorth01', name: 'Northbound' };

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

function video(id: string, channel: { id: string; name: string }) {
	return {
		type: 'video',
		id,
		title: `${channel.name} video ${id}`,
		thumbnails: [],
		channel: { ...channel, avatar: [], verified: false },
		duration_seconds: 600,
		view_count: 1200,
		published_at: null,
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

/** An in-memory blocks backend that filters search like the real one; `calls` records changes. */
async function mockApi(page: Page, { signedIn = true } = {}) {
	const state = {
		blocked: [] as { id: string; name: string; at: string }[],
		calls: [] as string[]
	};
	const isBlocked = (id: string) => state.blocked.some((b) => b.id === id);

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
			}
		})
	);
	await page.route('**/api/v0/auth/me', (route) =>
		signedIn ? json(route, account) : json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/subscriptions/*', (route) => json(route, { subscribed: false }));
	await page.route('**/api/v0/subscriptions', (route) => json(route, []));
	await page.route('**/api/v0/progress?*', (route) => json(route, []));
	await page.route('**/api/v0/search?*', (route) =>
		json(route, {
			items: [video('forge000001', FORGE), video('north000001', NORTH)].filter(
				(item) => !signedIn || !isBlocked(item.channel.id)
			),
			continuation: null,
			corrected_query: null
		})
	);
	await page.route(`**/api/v0/channels/${FORGE.id}`, (route) =>
		json(route, {
			...FORGE,
			handle: '@hammerandtongs',
			avatar: [],
			banner: [],
			verified: false,
			subscriber_count: 284000,
			video_count: 1,
			description: '',
			tags: [],
			has_shorts: false,
			has_live: false,
			videos: {
				items: isBlocked(FORGE.id) ? [] : [video('forge000001', FORGE)],
				continuation: null
			}
		})
	);
	await page.route(`**/api/v0/channels/${FORGE.id}/videos?*`, (route) =>
		json(route, {
			items: isBlocked(FORGE.id) ? [] : [video('forge000001', FORGE)],
			continuation: null
		})
	);
	await page.route('**/api/v0/blocks', (route) =>
		signedIn
			? json(
					route,
					state.blocked.map((b) => ({ id: b.id, name: b.name, avatar: [], blocked_at: b.at }))
				)
			: json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/blocks/*', (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		if (request.method() === 'PUT') {
			state.calls.push(`BLOCK ${id}`);
			const name = [FORGE, NORTH].find((c) => c.id === id)?.name ?? id;
			const at = '2026-11-26T10:00:00Z';
			if (!isBlocked(id)) state.blocked.unshift({ id, name, at });
			return json(route, { id, name, avatar: [], blocked_at: at }, 201);
		}
		if (request.method() === 'DELETE') {
			state.calls.push(`UNBLOCK ${id}`);
			state.blocked = state.blocked.filter((b) => b.id !== id);
			return route.fulfill({ status: 204 });
		}
		return route.fallback();
	});
	return state;
}

test('anonymous users are asked to sign in before blocking', async ({ page }) => {
	await mockApi(page, { signedIn: false });
	await page.goto('/results?search_query=forge');

	await page
		.getByRole('button', { name: `More actions for ${FORGE.name} video forge000001` })
		.click();
	await page.getByRole('menuitem', { name: 'Sign in to block channels' }).click();
	await expect(page).toHaveURL(/\/login\?redirect=%2Fresults%3Fsearch_query%3Dforge$/);
});

test('blocking from search hides the channel until it is unblocked', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/results?search_query=forge');

	const results = page.getByRole('heading', { level: 3 });
	await expect(results).toHaveText([/Hammer & Tongs video/, /Northbound video/]);

	await page
		.getByRole('button', { name: `More actions for ${FORGE.name} video forge000001` })
		.click();
	await page.getByRole('menuitem', { name: 'Block channel' }).click();
	await expect(results).toHaveText([/Northbound video/]);

	await page.goto(`/channel/${FORGE.id}`);
	const notice = page.getByTestId('channel-blocked');
	await expect(
		notice.getByRole('heading', { level: 1, name: `You blocked ${FORGE.name}` })
	).toBeVisible();
	await expect(page.getByRole('list', { name: 'Videos' })).toHaveCount(0);

	await notice.getByRole('link', { name: 'Manage blocked channels' }).click();
	await expect(page).toHaveURL(/\/settings\/blocked$/);
	const row = page.getByTestId('blocked-channel');
	await expect(row).toHaveCount(1);
	await expect(row.getByRole('link', { name: FORGE.name })).toBeVisible();
	await expect(page.getByText('1 blocked channel')).toBeVisible();

	// Unblocking here keeps the row, so it can be undone.
	await row.getByRole('button', { name: `Unblock ${FORGE.name}` }).click();
	await expect(row.getByTestId('reblock-button')).toBeVisible();
	await row.getByTestId('reblock-button').click();
	await expect(row.getByTestId('unblock-button')).toBeVisible();

	await page.goto(`/channel/${FORGE.id}`);
	await page.getByTestId('unblock-button').click();
	await expect(page.getByRole('heading', { level: 1, name: FORGE.name })).toBeVisible();
	await expect(page.getByRole('list', { name: 'Videos' }).getByRole('listitem')).toHaveCount(1);

	await page.goto('/results?search_query=forge');
	await expect(results).toHaveText([/Hammer & Tongs video/, /Northbound video/]);

	expect(state.calls).toEqual([
		`BLOCK ${FORGE.id}`,
		`UNBLOCK ${FORGE.id}`,
		`BLOCK ${FORGE.id}`,
		`UNBLOCK ${FORGE.id}`
	]);
});

test('the channel header offers to block the channel', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto(`/channel/${FORGE.id}`);

	await page.getByTestId('channel-actions').click();
	await page.getByRole('menuitem', { name: 'Block channel' }).click();
	await expect(page.getByTestId('channel-blocked')).toBeVisible();
	expect(state.calls).toEqual([`BLOCK ${FORGE.id}`]);
});
