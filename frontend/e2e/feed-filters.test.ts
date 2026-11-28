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

type Channel = { id: string; name: string };
type Video = { id: string; channel: Channel; short: boolean };

const VIDEOS: Video[] = [
	{ id: 'forge000001', channel: FORGE, short: false },
	{ id: 'north000001', channel: NORTH, short: false },
	{ id: 'forge000002', channel: FORGE, short: false },
	{ id: 'forgeshort1', channel: FORGE, short: true }
];

const title = (video: Video) => `${video.channel.name} video ${video.id}`;

function summary(video: Video) {
	return {
		id: video.id,
		title: title(video),
		thumbnails: [],
		channel: { ...video.channel, avatar: [], verified: false },
		duration_seconds: video.short ? 30 : 600,
		view_count: 1200,
		published_at: '2026-11-20T10:00:00Z',
		published_text: null,
		is_live: false,
		is_short: video.short,
		is_upcoming: false
	};
}

function subscription(channel: Channel) {
	return {
		...channel,
		handle: null,
		avatar: [],
		subscriber_count: 1000,
		verified: false,
		subscribed_at: '2026-11-20T10:00:00Z'
	};
}

/** An in-memory backend that filters the feed like the real one; `calls` records changes. */
async function mockApi(page: Page, { signedIn = true } = {}) {
	const state = {
		watched: new Set<string>(),
		hidden: new Set<string>(),
		settings: {
			video_codecs: [],
			max_quality: null,
			default_quality: null,
			home_sections: null,
			audio_language: null,
			feed_hide_watched: false,
			feed_hide_shorts: false
		},
		calls: [] as string[],
		feedQueries: [] as string[]
	};
	const progress = (id: string) => ({
		video_id: id,
		position_seconds: 600,
		duration_seconds: 600,
		finished: true,
		updated_at: '2026-11-26T10:00:00Z'
	});
	const unauthorized = (route: Route) => json(route, { code: 'unauthorized', message: '' }, 401);

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
		signedIn ? json(route, account) : unauthorized(route)
	);
	await page.route('**/api/v0/subscriptions', (route) =>
		json(route, [subscription(FORGE), subscription(NORTH)])
	);
	await page.route('**/api/v0/settings', (route) => {
		if (!signedIn) return unauthorized(route);
		if (route.request().method() === 'PATCH') {
			const patch = route.request().postDataJSON() as Record<string, unknown>;
			state.calls.push(`PATCH ${JSON.stringify(patch)}`);
			Object.assign(state.settings, patch);
		}
		return json(route, state.settings);
	});
	await page.route('**/api/v0/feed?*', (route) => {
		const params = new URL(route.request().url()).searchParams;
		const kind = params.get('kind') ?? 'all';
		const channel = params.get('channel');
		const hideWatched = params.get('hide_watched') === 'true';
		state.feedQueries.push(params.toString());
		const items = VIDEOS.filter(
			(v) =>
				(kind === 'all' || v.short === (kind === 'shorts')) &&
				(!channel || v.channel.id === channel) &&
				!(hideWatched && state.watched.has(v.id)) &&
				!state.hidden.has(v.id)
		).map(summary);
		return json(route, { items, continuation: null });
	});
	await page.route('**/api/v0/search?*', (route) =>
		json(route, {
			items: VIDEOS.map((v) => ({ type: 'video', ...summary(v) })),
			continuation: null,
			corrected_query: null
		})
	);
	await page.route('**/api/v0/progress?*', (route) => {
		const ids = new URL(route.request().url()).searchParams.get('ids')?.split(',') ?? [];
		return json(route, ids.filter((id) => state.watched.has(id)).map(progress));
	});
	await page.route('**/api/v0/progress/*', (route) => {
		const id = new URL(route.request().url()).pathname.split('/').pop() ?? '';
		return json(route, { progress: state.watched.has(id) ? progress(id) : null });
	});
	await page.route('**/api/v0/history/*', (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		if (request.method() === 'PUT') {
			state.calls.push(`WATCHED ${id} ${JSON.stringify(request.postDataJSON())}`);
			state.watched.add(id);
			return json(route, progress(id));
		}
		if (request.method() === 'DELETE') {
			state.calls.push(`UNWATCHED ${id}`);
			state.watched.delete(id);
			return route.fulfill({ status: 204 });
		}
		return route.fallback();
	});
	await page.route('**/api/v0/hidden-videos/*', (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		state.calls.push(`${request.method() === 'PUT' ? 'HIDE' : 'UNHIDE'} ${id}`);
		if (request.method() === 'PUT') state.hidden.add(id);
		else state.hidden.delete(id);
		return route.fulfill({ status: 204 });
	});
	return state;
}

const menuFor = (page: Page, video: Video) =>
	page.getByRole('button', { name: `More actions for ${title(video)}` });

test('guests are offered neither marking nor hiding', async ({ page }) => {
	await mockApi(page, { signedIn: false });
	await page.goto('/results?search_query=forge');

	await menuFor(page, VIDEOS[0]).click();
	await expect(page.getByRole('menuitem', { name: 'Play next' })).toBeVisible();
	await expect(page.getByTestId('mark-watched')).toHaveCount(0);
	await expect(page.getByTestId('not-interested')).toHaveCount(0);
});

test('feed filters are applied by the server and remembered', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/subscriptions');

	const grid = page.getByRole('list', { name: 'Subscriptions' });
	const shelf = page.getByRole('heading', { level: 2, name: 'Shorts' });
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([
		/forge000001/,
		/north000001/,
		/forge000002/
	]);
	await expect(shelf).toBeVisible();

	await menuFor(page, VIDEOS[0]).click();
	await page.getByRole('menuitem', { name: 'Mark as watched' }).click();
	await expect.poll(() => state.calls).toEqual(['WATCHED forge000001 {"duration_seconds":600}']);

	await page.getByTestId('feed-filter-watched').click();
	await expect(page.getByTestId('feed-filter-watched')).toHaveAttribute('aria-pressed', 'true');
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([/north000001/, /forge000002/]);
	await expect
		.poll(() => state.feedQueries)
		.toContainEqual(expect.stringContaining('hide_watched=true'));

	await page.getByTestId('feed-filter-shorts').click();
	await expect(shelf).toHaveCount(0);

	await page.getByTestId('feed-filter-channel').click();
	await page.getByTestId('feed-filter-channel-search').fill('north');
	await page.getByTestId('feed-filter-channels').getByRole('button', { name: NORTH.name }).click();
	await expect(page).toHaveURL(new RegExp(`channel=${NORTH.id}$`));
	await expect(page.getByTestId('feed-filter-channel')).toHaveAccessibleName(
		`Channel: ${NORTH.name}`
	);
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([/north000001/]);

	await expect
		.poll(() => state.calls.slice(1))
		.toEqual(['PATCH {"feed_hide_watched":true}', 'PATCH {"feed_hide_shorts":true}']);

	// The toggles come back from the settings, the channel from the URL.
	await page.goto('/feed/subscriptions');
	await expect(page.getByTestId('feed-filter-watched')).toHaveAttribute('aria-pressed', 'true');
	await expect(page.getByTestId('feed-filter-shorts')).toHaveAttribute('aria-pressed', 'true');
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([/north000001/, /forge000002/]);

	await page.getByTestId('feed-filters-clear').click();
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
	await expect(shelf).toBeVisible();
});

test('marking a video in a feed that hides watched videos drops it, unmarking brings it back', async ({
	page
}) => {
	const state = await mockApi(page);
	state.settings.feed_hide_watched = true;
	await page.goto('/feed/subscriptions');

	const grid = page.getByRole('list', { name: 'Subscriptions' });
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
	await menuFor(page, VIDEOS[1]).click();
	await page.getByRole('menuitem', { name: 'Mark as watched' }).click();
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([/forge000001/, /forge000002/]);

	await page.goto('/results?search_query=north');
	await menuFor(page, VIDEOS[1]).click();
	await page.getByRole('menuitem', { name: 'Mark as unwatched' }).click();
	await expect
		.poll(() => state.calls)
		.toEqual(['WATCHED north000001 {"duration_seconds":600}', 'UNWATCHED north000001']);

	await page.goto('/feed/subscriptions');
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
});

test('not interested hides a video from the feed until undone', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/subscriptions');

	const grid = page.getByRole('list', { name: 'Subscriptions' });
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
	await menuFor(page, VIDEOS[2]).click();
	await page.getByRole('menuitem', { name: 'Not interested' }).click();
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([/forge000001/, /north000001/]);

	await page.getByRole('button', { name: 'Undo' }).click();
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
	await expect.poll(() => state.calls).toEqual(['HIDE forge000002', 'UNHIDE forge000002']);
});
