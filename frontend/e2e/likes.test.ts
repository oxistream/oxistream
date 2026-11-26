import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

const VIDEOS = ['vidAAAAAAA1', 'vidBBBBBBB2'];
const SHORT = 'sho0000001A';
const MPD = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011" type="static" mediaPresentationDuration="PT60S" minBufferTime="PT1.5S"><Period></Period></MPD>`;

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

function summary(id: string) {
	return {
		id,
		title: id === SHORT ? 'Forge short' : `Fjord drive ${VIDEOS.indexOf(id) + 1}`,
		thumbnails: [],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: 600,
		view_count: null,
		published_at: null,
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

/** An in-memory likes backend; `calls` records every change in order. */
async function mockApi(page: Page, { signedIn = true } = {}) {
	const state = { liked: [] as { id: string; at: string }[], calls: [] as string[] };
	let clock = Date.parse('2026-11-26T10:00:00Z');

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
	await page.route('**/api/v0/playlist-follows', (route) => json(route, []));
	await page.route(/\/api\/v0\/playlists(\?.*)?$/, (route) => json(route, []));
	await page.route(/\/api\/v0\/progress\/[^/]+$/, (route) => json(route, { progress: null }));
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({ status: 200, contentType: 'application/dash+xml', body: MPD })
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		return json(route, {
			...summary(id),
			description: '',
			like_count: id === SHORT ? null : 18420,
			keywords: [],
			subtitles: [],
			related: []
		});
	});
	await page.route(/\/api\/v0\/likes(\?.*)?$/, (route) =>
		json(route, {
			items: state.liked.map((l) => ({ video: summary(l.id), liked_at: l.at })),
			total: state.liked.length,
			continuation: null
		})
	);
	await page.route(/\/api\/v0\/likes\/[^/]+$/, (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		const existing = state.liked.find((l) => l.id === id);
		if (request.method() === 'PUT') {
			state.calls.push(`LIKE ${id}`);
			if (!existing) {
				clock += 60_000;
				state.liked.unshift({ id, at: new Date(clock).toISOString() });
			}
			return json(route, { liked: true, liked_at: new Date(clock).toISOString() }, 201);
		}
		if (request.method() === 'DELETE') {
			state.calls.push(`UNLIKE ${id}`);
			state.liked = state.liked.filter((l) => l.id !== id);
			return route.fulfill({ status: 204 });
		}
		return json(route, { liked: !!existing, liked_at: existing?.at ?? null });
	});
	return state;
}

const likeButton = (page: Page) => page.getByTestId('like-button');

test('anonymous users are asked to sign in', async ({ page }) => {
	await mockApi(page, { signedIn: false });

	await page.goto(`/watch?v=${VIDEOS[0]}`);
	await expect(likeButton(page)).toHaveAttribute(
		'href',
		`/login?redirect=%2Fwatch%3Fv%3D${VIDEOS[0]}`
	);
	await expect(likeButton(page)).toHaveText(/18K/);

	await page.goto('/playlist?list=LL');
	await expect(page.getByText('Sign in to see the videos you liked.')).toBeVisible();
});

test('liked videos and shorts form a list in the library', async ({ page }) => {
	const state = await mockApi(page);

	await page.goto(`/watch?v=${VIDEOS[0]}`);
	await expect(likeButton(page)).toHaveAttribute('aria-pressed', 'false');
	await likeButton(page).click();
	await expect(likeButton(page)).toHaveAttribute('aria-pressed', 'true');

	await page.goto(`/watch?v=${VIDEOS[1]}`);
	await likeButton(page).click();
	await expect(likeButton(page)).toHaveAttribute('aria-pressed', 'true');

	await page.goto(`/shorts/${SHORT}`);
	const short = page.locator(`[data-short-id="${SHORT}"][data-active]`).getByTestId('like-button');
	await expect(short).toHaveAttribute('aria-pressed', 'false');
	await short.click();
	await expect(short).toHaveAttribute('aria-pressed', 'true');

	await page.goto('/feed/playlists');
	const card = page.getByTestId('liked-playlist-card');
	await expect(card.getByRole('link', { name: /Liked videos.*3 videos/ })).toHaveAttribute(
		'href',
		'/playlist?list=LL'
	);
	await card.getByRole('link').click();

	await expect(page.getByRole('heading', { level: 1, name: 'Liked videos' })).toBeVisible();
	const titles = page.getByTestId('playlist-item').getByRole('heading', { level: 3 });
	await expect(titles).toHaveText([/Forge short/, /Fjord drive 2/, /Fjord drive 1/]);
	await expect(page.getByRole('button', { name: /^Reorder/ })).toHaveCount(0);
	await expect(page.getByRole('link', { name: 'Play all' })).toHaveAttribute(
		'href',
		`/watch?v=${SHORT}&list=LL`
	);

	await page.getByRole('button', { name: 'Actions for Fjord drive 2' }).click();
	await page.getByRole('menuitem', { name: 'Remove from liked videos' }).click();
	await expect(titles).toHaveText([/Forge short/, /Fjord drive 1/]);

	await page.goto(`/watch?v=${VIDEOS[0]}&list=LL`);
	const panel = page.getByTestId('playlist-panel');
	await expect(panel.getByText('Liked videos')).toBeVisible();
	await expect(likeButton(page)).toHaveAttribute('aria-pressed', 'true');
	await likeButton(page).click();
	await expect(likeButton(page)).toHaveAttribute('aria-pressed', 'false');

	expect(state.calls).toEqual([
		`LIKE ${VIDEOS[0]}`,
		`LIKE ${VIDEOS[1]}`,
		`LIKE ${SHORT}`,
		`UNLIKE ${VIDEOS[1]}`,
		`UNLIKE ${VIDEOS[0]}`
	]);
});
