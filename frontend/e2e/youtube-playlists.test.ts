import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const LIST = 'PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI';
const PAGE_SIZES = [100, 100, 30];
const TOTAL = 232;
const MPD = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011" type="static" mediaPresentationDuration="PT60S" minBufferTime="PT1.5S"><Period></Period></MPD>`;

const videoId = (n: number) => `yt${String(n).padStart(8, '0')}x`;

function summary(n: number) {
	return {
		id: videoId(n),
		title: `Coast ${n}`,
		thumbnails: [],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: 300 + n,
		view_count: 1000,
		published_at: null,
		published_text: '2 years ago',
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

/** Item numbers (1-based) of each page. */
function pageItems(index: number) {
	const start = PAGE_SIZES.slice(0, index).reduce((sum, size) => sum + size, 0);
	return Array.from({ length: PAGE_SIZES[index] }, (_, i) => summary(start + i + 1));
}

const continuation = (index: number) => (index < PAGE_SIZES.length ? `page-${index}` : null);

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};
const COPY_ID = 'Cp9xQ2mKp9LsVb3N';
let calls: string[] = [];

/**
 * A three-page YouTube playlist; `requests` records every playlist call in order, `calls` every
 * follow and import request.
 */
async function mockApi(page: Page, options: { signedIn?: boolean; following?: boolean } = {}) {
	const requests: string[] = [];
	calls = [];
	let following = options.following ?? false;
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
		options.signedIn
			? json(route, account)
			: json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route(/\/api\/v0\/playlist-follows\/[^/]+(\/seen)?$/, (route) => {
		const method = route.request().method();
		const seen = route.request().url().endsWith('/seen');
		if (method !== 'GET') calls.push(`${method} ${seen ? 'seen' : 'follow'}`);
		if (seen) return route.fulfill({ status: following ? 204 : 404 });
		if (method === 'GET') return json(route, { following });
		following = method === 'PUT';
		if (!following) return route.fulfill({ status: 204 });
		return json(
			route,
			{
				id: LIST,
				title: null,
				thumbnails: [],
				new_video_count: 0,
				unavailable: false,
				followed_at: '2026-11-26T10:00:00Z'
			},
			201
		);
	});
	await page.route('**/api/v0/playlists', (route) => json(route, []));
	await page.route('**/api/v0/playlist-follows', (route) =>
		json(
			route,
			following
				? [
						{
							id: LIST,
							title: 'Coastal roads',
							channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
							thumbnails: [],
							video_count: TOTAL,
							new_video_count: 3,
							unavailable: false,
							followed_at: '2026-11-20T10:00:00Z'
						}
					]
				: []
		)
	);
	await page.route('**/api/v0/playlists/import', (route) => {
		const body = route.request().postDataJSON() as { playlist_id: string };
		calls.push(`import ${body.playlist_id}`);
		return json(
			route,
			{
				id: COPY_ID,
				title: 'Coastal roads',
				description: '',
				visibility: 'private',
				item_count: 230,
				thumbnails: [],
				created_at: '2026-11-26T10:00:00Z',
				updated_at: '2026-11-26T10:00:00Z'
			},
			201
		);
	});
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({ status: 200, contentType: 'application/dash+xml', body: MPD })
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		return json(route, {
			...summary(Number(id.slice(2, 10))),
			id,
			description: '',
			like_count: null,
			keywords: [],
			subtitles: [],
			related: []
		});
	});
	await page.route(/\/api\/v0\/youtube\/playlists\/[^/]+(\/videos)?(\?.*)?$/, (route) => {
		const url = new URL(route.request().url());
		const [, , , , , id, videos] = url.pathname.split('/');
		const token = url.searchParams.get('continuation');
		requests.push(videos ? `videos ${token}` : 'playlist');
		if (id !== LIST) return json(route, { code: 'not_found', message: 'not found' }, 404);
		if (videos) {
			const index = Number(token?.split('-')[1]);
			return json(route, { items: pageItems(index), continuation: continuation(index + 1) });
		}
		return json(route, {
			id: LIST,
			title: 'Coastal roads',
			description: 'Every drive along the coast',
			channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
			video_count: TOTAL,
			thumbnails: [],
			last_update: '2026-11-24',
			last_update_text: 'Last updated on Nov 24, 2026',
			videos: { items: pageItems(0), continuation: continuation(1) }
		});
	});
	return requests;
}

test('a YouTube playlist renders read-only and loads more on scroll', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/playlist?list=${LIST}`);

	await expect(page.getByRole('heading', { level: 1, name: 'Coastal roads' })).toBeVisible();
	await expect(page).toHaveTitle(/Coastal roads/);
	const hero = page.getByRole('complementary');
	await expect(hero.getByRole('link', { name: 'Northbound' })).toHaveAttribute(
		'href',
		'/channel/UCnorth'
	);
	await expect(hero.getByText('YouTube playlist')).toBeVisible();
	await expect(hero.getByText('232 videos', { exact: true }).last()).toBeVisible();
	await expect(hero.getByText(/^Updated /)).toBeVisible();
	await expect(page.getByRole('link', { name: 'Play all' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(1)}&list=${LIST}`
	);
	await expect(page.getByRole('button', { name: 'Edit playlist' })).toHaveCount(0);
	await expect(page.getByRole('button', { name: /^Reorder/ })).toHaveCount(0);
	await expect(page.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);

	const items = page.getByTestId('playlist-item');
	await expect(items).toHaveCount(100);
	await items.last().scrollIntoViewIfNeeded();
	await expect(items).toHaveCount(200);
	await items.last().scrollIntoViewIfNeeded();
	await expect(items).toHaveCount(230);
	await expect(page.getByText('Unavailable videos are hidden.')).toBeVisible();
	await expect(items.last().getByRole('link', { name: /Coast 230/ })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(230)}&list=${LIST}`
	);
	expect(requests).toEqual(['playlist', 'videos page-1', 'videos page-2']);
});

test('the watch page loads the next page and plays on into it', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/watch?v=${videoId(100)}&list=${LIST}`);

	const panel = page.getByTestId('playlist-panel');
	await expect(panel.getByText(`100 / ${TOTAL}`)).toBeVisible();
	await expect(panel.getByRole('link', { name: /Coast 100/ })).toHaveAttribute(
		'aria-current',
		'true'
	);
	await expect(panel.getByRole('link', { name: 'Next video' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(101)}&list=${LIST}`
	);
	expect(requests).toEqual(['playlist', 'videos page-1']);

	await page.locator('dash-video').dispatchEvent('ended');
	await expect(page).toHaveURL(`/watch?v=${videoId(101)}&list=${LIST}`);
	await expect(panel.getByText(`101 / ${TOTAL}`)).toBeVisible();
	await expect(panel.getByRole('link', { name: 'Previous video' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(100)}&list=${LIST}`
	);
});

test('a deep link finds its video on a later page', async ({ page }) => {
	const requests = await mockApi(page);
	await page.goto(`/watch?v=${videoId(229)}&list=${LIST}`);

	const panel = page.getByTestId('playlist-panel');
	await expect(panel.getByText(`229 / 230`)).toBeVisible();
	await expect(panel.getByRole('link', { name: 'Next video' })).toHaveAttribute(
		'href',
		`/watch?v=${videoId(230)}&list=${LIST}`
	);
	expect(requests).toEqual(['playlist', 'videos page-1', 'videos page-2']);
});

test('mixes and unknown playlists get their own states', async ({ page }) => {
	const requests = await mockApi(page);

	await page.goto('/playlist?list=RDdQw4w9WgXcQ');
	await expect(
		page.getByRole('heading', { name: "Mixes can't be opened as playlists" })
	).toBeVisible();

	await page.goto(`/watch?v=${videoId(1)}&list=RDdQw4w9WgXcQ`);
	await expect(
		page.getByTestId('playlist-panel').getByText("Mixes can't be opened as playlists")
	).toBeVisible();
	expect(requests).toEqual([]);

	await page.goto('/playlist?list=PLzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz');
	await expect(page.getByRole('heading', { name: 'Playlist not found' })).toBeVisible();

	await page.goto(`/watch?v=${videoId(1)}&list=PLzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz`);
	await expect(page.getByTestId('playlist-panel').getByText('Playlist unavailable')).toBeVisible();
});

test('anonymous users are sent to sign in to follow or copy', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/playlist?list=${LIST}`);

	const redirect = `/login?redirect=${encodeURIComponent(`/playlist?list=${LIST}`)}`;
	await expect(page.getByRole('link', { name: 'Follow' })).toHaveAttribute('href', redirect);
	await expect(page.getByRole('link', { name: 'Save as copy' })).toHaveAttribute('href', redirect);
});

test('a YouTube playlist can be followed and saved as a copy', async ({ page }) => {
	await mockApi(page, { signedIn: true });
	await page.goto(`/playlist?list=${LIST}`);

	const follow = page.getByTestId('follow-button');
	await expect(follow).toHaveText('Follow');
	await follow.click();
	await expect(follow).toHaveText('Following');
	await expect(follow).toHaveAttribute('aria-pressed', 'true');
	await expect.poll(() => calls).toEqual(['PUT follow']);
	await follow.click();
	await expect(follow).toHaveText('Follow');
	await expect.poll(() => calls).toEqual(['PUT follow', 'DELETE follow']);

	await page.getByRole('button', { name: 'Save as copy' }).click();
	await expect(page).toHaveURL(`/playlist?list=${COPY_ID}`);
	expect(calls).toContain(`import ${LIST}`);
});

test('followed playlists show in the library and opening one marks it seen', async ({ page }) => {
	await mockApi(page, { signedIn: true, following: true });
	await page.goto('/feed/playlists');

	const section = page.getByRole('region', { name: 'Followed on YouTube' });
	await expect(section.getByText('Northbound')).toBeVisible();
	await expect(section.getByText('3 new videos', { exact: true })).toBeVisible();
	await expect(page.getByText('No playlists of your own yet.')).toBeVisible();

	await section.getByRole('link', { name: /Coastal roads/ }).click();
	await expect(page).toHaveURL(`/playlist?list=${LIST}`);
	await expect(page.getByTestId('follow-button')).toHaveText('Following');
	await expect.poll(() => calls).toEqual(['PUT seen']);
});
