import { expect, test, type Page, type Route } from '@playwright/test';

test.use({ locale: 'en-US' });

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	created_at: '2026-11-26T10:00:00Z'
};

const LIST = 'Rd7xQ2mKp9LsVb3N';
const VIDEOS = ['vidAAAAAAA1', 'vidBBBBBBB2', 'vidCCCCCCC3'];
const MPD = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011" type="static" mediaPresentationDuration="PT60S" minBufferTime="PT1.5S"><Period></Period></MPD>`;

function summary(id: string, i: number) {
	return {
		id,
		title: `Fjord drive ${i + 1}`,
		thumbnails: [],
		channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
		duration_seconds: 600 + i,
		view_count: null,
		published_at: null,
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

type Item = ReturnType<typeof summary>;
type MockPlaylist = {
	id: string;
	title: string;
	description: string;
	visibility: 'private' | 'unlisted';
	items: Item[];
	last_video_id: string | null;
};

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

/** An in-memory playlist backend; `calls` records every change in order. */
async function mockApi(page: Page, options: { signedIn?: boolean; owner?: boolean } = {}) {
	const signedIn = options.signedIn ?? true;
	const owner = options.owner ?? signedIn;
	const state = {
		playlists: [
			{
				id: LIST,
				title: 'Road trips',
				description: 'Long drives along the coast',
				visibility: 'unlisted',
				items: VIDEOS.map(summary),
				last_video_id: null
			}
		] as MockPlaylist[],
		calls: [] as string[]
	};
	const toDto = (p: MockPlaylist, videoId?: string | null) => ({
		id: p.id,
		title: p.title,
		description: p.description,
		visibility: p.visibility,
		item_count: p.items.length,
		thumbnails: [],
		last_video_id: owner ? p.last_video_id : null,
		...(videoId ? { contains_video: p.items.some((i) => i.id === videoId) } : {}),
		created_at: '2026-11-20T10:00:00Z',
		updated_at: '2026-11-25T10:00:00Z'
	});
	const find = (id: string) => state.playlists.find((p) => p.id === id);

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
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({ status: 200, contentType: 'application/dash+xml', body: MPD })
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		const index = Math.max(0, VIDEOS.indexOf(id));
		return json(route, {
			...summary(id, index),
			description: '',
			channel: { id: 'UCnorth', name: 'Northbound', avatar: [], verified: false },
			like_count: null,
			keywords: [],
			subtitles: [],
			related: []
		});
	});
	await page.route(/\/api\/v0\/playlists(\?.*)?$/, async (route) => {
		const request = route.request();
		if (request.method() === 'POST') {
			const body = request.postDataJSON() as Partial<MockPlaylist>;
			const created: MockPlaylist = {
				id: 'Nw4pL8sTq1ZcXy6M',
				title: body.title ?? '',
				description: body.description ?? '',
				visibility: body.visibility ?? 'private',
				items: [],
				last_video_id: null
			};
			state.playlists.unshift(created);
			state.calls.push(`POST ${created.title}`);
			return json(route, toDto(created), 201);
		}
		const videoId = new URL(request.url()).searchParams.get('video_id');
		return json(
			route,
			state.playlists.map((p) => toDto(p, videoId))
		);
	});
	await page.route(/\/api\/v0\/playlists\/[^/]+$/, async (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		const playlist = find(id);
		if (!playlist || (!owner && playlist.visibility !== 'unlisted')) {
			return json(route, { code: 'not_found', message: '' }, 404);
		}
		if (request.method() === 'DELETE') {
			state.playlists = state.playlists.filter((p) => p.id !== id);
			state.calls.push(`DELETE ${id}`);
			return route.fulfill({ status: 204 });
		}
		if (request.method() === 'PATCH') {
			Object.assign(playlist, request.postDataJSON() as Partial<MockPlaylist>);
			state.calls.push(`PATCH ${id}`);
			return json(route, toDto(playlist));
		}
		return json(route, {
			...toDto(playlist),
			owner: 'mara',
			is_owner: owner,
			items: playlist.items.map((video) => ({ video, added_at: '2026-11-21T10:00:00Z' }))
		});
	});
	await page.route(/\/api\/v0\/playlists\/[^/]+\/items\/[^/]+(\/position)?$/, async (route) => {
		const request = route.request();
		const [, , , , id, , videoId, position] = new URL(request.url()).pathname.split('/');
		const playlist = find(id);
		if (!playlist) return json(route, { code: 'not_found', message: '' }, 404);
		const from = playlist.items.findIndex((i) => i.id === videoId);
		if (position) {
			const { index } = request.postDataJSON() as { index: number };
			const [item] = playlist.items.splice(from, 1);
			playlist.items.splice(index, 0, item);
			state.calls.push(`MOVE ${videoId} ${index}`);
			return route.fulfill({ status: 204 });
		}
		if (request.method() === 'PUT') {
			if (from < 0) playlist.items.push(summary(videoId, playlist.items.length));
			state.calls.push(`ADD ${id} ${videoId}`);
			return json(route, toDto(playlist), from < 0 ? 201 : 200);
		}
		if (from >= 0) playlist.items.splice(from, 1);
		state.calls.push(`REMOVE ${id} ${videoId}`);
		return route.fulfill({ status: 204 });
	});
	await page.route(/\/api\/v0\/playlists\/[^/]+\/last-played$/, async (route) => {
		const id = new URL(route.request().url()).pathname.split('/')[4];
		const playlist = find(id);
		const { video_id } = route.request().postDataJSON() as { video_id: string };
		if (playlist) playlist.last_video_id = video_id;
		state.calls.push(`LAST ${video_id}`);
		return route.fulfill({ status: 204 });
	});
	return state;
}

const titles = (page: Page) =>
	page
		.getByTestId('playlist-item')
		.getByRole('heading', { level: 3 })
		.evaluateAll((headings) => headings.map((h) => h.textContent?.split(',')[0].trim()));

test('anonymous users get a sign-in prompt but can open unlisted playlists', async ({ page }) => {
	await mockApi(page, { signedIn: false });

	await page.goto('/feed/playlists');
	await expect(page.getByRole('heading', { level: 1, name: 'Playlists' })).toBeVisible();
	await expect(page.getByRole('main').getByRole('link', { name: 'Sign in' })).toBeVisible();

	await page.goto(`/playlist?list=${LIST}`);
	await expect(page.getByRole('heading', { level: 1, name: 'Road trips' })).toBeVisible();
	await expect(page.getByTestId('playlist-item')).toHaveCount(3);
	await expect(page.getByRole('button', { name: 'Edit playlist' })).toHaveCount(0);
	await expect(page.getByRole('button', { name: /^Reorder/ })).toHaveCount(0);
	await expect(page.getByRole('link', { name: 'Play all' })).toHaveAttribute(
		'href',
		`/watch?v=${VIDEOS[0]}&list=${LIST}`
	);

	await page.goto(`/watch?v=${VIDEOS[0]}`);
	await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
});

test('unknown playlists get a not-found state', async ({ page }) => {
	await mockApi(page);

	await page.goto('/playlist?list=Zz9Zz9Zz9Zz9Zz9Z');
	await expect(page.getByRole('heading', { name: 'Playlist not found' })).toBeVisible();
});

test('creating a playlist opens it', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/playlists');
	await expect(page.getByRole('link', { name: /Road trips/ })).toHaveAttribute(
		'href',
		`/playlist?list=${LIST}`
	);

	await page.getByRole('button', { name: 'New playlist' }).click();
	const dialog = page.getByRole('dialog');
	await dialog.getByRole('button', { name: 'Create' }).click();
	await expect(dialog.getByText('Enter a title.')).toBeVisible();
	await dialog.getByLabel('Title').fill('Winter hikes');
	await dialog.getByRole('button', { name: 'Create' }).click();

	await expect(page).toHaveURL(/\/playlist\?list=Nw4pL8sTq1ZcXy6M$/);
	await expect(page.getByRole('heading', { level: 1, name: 'Winter hikes' })).toBeVisible();
	await expect(page.getByText('This playlist is empty')).toBeVisible();
	expect(state.calls).toEqual(['POST Winter hikes']);
});

test('owners reorder, remove and delete', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto(`/playlist?list=${LIST}`);
	await expect(page.getByTestId('playlist-item')).toHaveCount(3);

	await page.getByRole('button', { name: 'Reorder Fjord drive 3' }).focus();
	await page.keyboard.press('ArrowUp');
	await expect
		.poll(() => titles(page))
		.toEqual(['Fjord drive 1', 'Fjord drive 3', 'Fjord drive 2']);
	await expect(page.getByRole('button', { name: 'Reorder Fjord drive 3' })).toBeFocused();

	await page.getByRole('button', { name: 'Actions for Fjord drive 1' }).click();
	await page.getByRole('menuitem', { name: 'Move to bottom' }).click();
	await expect
		.poll(() => titles(page))
		.toEqual(['Fjord drive 3', 'Fjord drive 2', 'Fjord drive 1']);

	await page.getByRole('button', { name: 'Actions for Fjord drive 2' }).click();
	await page.getByRole('menuitem', { name: 'Remove' }).click();
	await expect.poll(() => titles(page)).toEqual(['Fjord drive 3', 'Fjord drive 1']);

	await page.getByRole('button', { name: 'Delete playlist' }).click();
	await page.getByRole('alertdialog').getByRole('button', { name: 'Delete playlist' }).click();
	await expect(page).toHaveURL(/\/feed\/playlists$/);
	await expect(page.getByText('No playlists of your own yet.')).toBeVisible();

	expect(state.calls).toEqual([
		`MOVE ${VIDEOS[2]} 1`,
		`MOVE ${VIDEOS[0]} 2`,
		`REMOVE ${LIST} ${VIDEOS[1]}`,
		`DELETE ${LIST}`
	]);
});

test('the watch page saves to playlists and plays through one', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto(`/watch?v=${VIDEOS[1]}&list=${LIST}`);

	const panel = page.getByTestId('playlist-panel');
	await expect(panel.getByText('2 / 3')).toBeVisible();
	await expect(panel.getByRole('link', { name: /Fjord drive 2/ })).toHaveAttribute(
		'aria-current',
		'true'
	);
	await expect(panel.getByRole('link', { name: 'Next video' })).toHaveAttribute(
		'href',
		`/watch?v=${VIDEOS[2]}&list=${LIST}`
	);
	await expect.poll(() => state.calls).toContain(`LAST ${VIDEOS[1]}`);

	await page.getByRole('button', { name: 'Save', exact: true }).click();
	const dialog = page.getByRole('dialog');
	const roadTrips = dialog.getByRole('checkbox', { name: 'Road trips' });
	await expect(roadTrips).toBeChecked();
	await roadTrips.click();
	await expect(roadTrips).not.toBeChecked();

	await dialog.getByRole('button', { name: 'New playlist' }).click();
	await dialog.getByLabel('Title').fill('Later');
	await dialog.getByRole('button', { name: 'Create' }).click();
	await expect(dialog.getByRole('checkbox', { name: 'Later' })).toBeChecked();

	expect(state.calls).toEqual([
		`LAST ${VIDEOS[1]}`,
		`REMOVE ${LIST} ${VIDEOS[1]}`,
		'POST Later',
		`ADD Nw4pL8sTq1ZcXy6M ${VIDEOS[1]}`
	]);
});

test('the playlist page continues where the owner left off', async ({ page }) => {
	const state = await mockApi(page);
	state.playlists[0].last_video_id = VIDEOS[1];
	await page.goto(`/playlist?list=${LIST}`);

	await expect(page.getByTestId('playlist-continue')).toHaveAttribute(
		'href',
		`/watch?v=${VIDEOS[1]}&list=${LIST}`
	);
	await expect(page.getByRole('link', { name: 'Play from start' })).toHaveAttribute(
		'href',
		`/watch?v=${VIDEOS[0]}&list=${LIST}`
	);
});

test('the header actions fit one row on a German phone', async ({ page }) => {
	await page.addInitScript(() => localStorage.setItem('PARAGLIDE_LOCALE', 'de'));
	const state = await mockApi(page);
	state.playlists[0].last_video_id = VIDEOS[1];
	await page.setViewportSize({ width: 360, height: 780 });
	await page.goto(`/playlist?list=${LIST}`);

	const actions = [
		page.getByTestId('playlist-continue'),
		page.getByRole('link', { name: 'Von vorn abspielen' }),
		page.getByRole('button', { name: 'Link kopieren' }),
		page.getByRole('button', { name: 'Playlist bearbeiten' }),
		page.getByRole('button', { name: 'Playlist löschen' })
	];
	for (const action of actions) await expect(action).toBeVisible();
	const tops = await Promise.all(actions.map(async (a) => Math.round((await a.boundingBox())!.y)));
	expect(new Set(tops).size).toBe(1);

	await page.getByRole('button', { name: 'Playlist bearbeiten' }).hover();
	await expect(page.locator('[data-slot="tooltip-content"]')).toHaveText('Playlist bearbeiten');
});

test('the header sits beside the list on wide screens and above it on phones', async ({ page }) => {
	await mockApi(page);
	await page.setViewportSize({ width: 1440, height: 900 });
	await page.goto(`/playlist?list=${LIST}`);
	const hero = page.getByRole('complementary');
	const list = page.getByRole('list', { name: 'Videos in this playlist' });
	await expect(list.getByTestId('playlist-item')).toHaveCount(3);

	const wideHero = (await hero.boundingBox())!;
	const wideList = (await list.boundingBox())!;
	expect(wideHero.x + wideHero.width).toBeLessThan(wideList.x);
	expect(Math.abs(wideHero.y - wideList.y)).toBeLessThan(8);
	await expect(hero).toHaveCSS('position', 'sticky');
	const main = (await page.getByRole('main').boundingBox())!;
	const left = wideHero.x - main.x;
	const right = main.x + main.width - (wideList.x + wideList.width);
	expect(Math.abs(left - right)).toBeLessThan(32);

	await page.setViewportSize({ width: 390, height: 844 });
	await expect(hero).toHaveCSS('position', 'static');
	const phoneHero = (await hero.boundingBox())!;
	const phoneList = (await list.boundingBox())!;
	expect(phoneHero.y + phoneHero.height).toBeLessThanOrEqual(phoneList.y);
	expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
});
