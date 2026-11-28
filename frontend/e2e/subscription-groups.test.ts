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
const TIDE = { id: 'UCtidetidetidetidetide01', name: 'Tidewater' };
const CHANNELS = [FORGE, NORTH, TIDE];

const VIDEOS = [
	{ id: 'forge000001', channel: FORGE },
	{ id: 'north000001', channel: NORTH },
	{ id: 'tide0000001', channel: TIDE }
];

const WORKSHOP = '01a2204e-0000-7000-8000-000000000001';
const TRAVEL = '01a2204e-0000-7000-8000-000000000002';

type Group = { id: string; name: string; show_on_home: boolean; channel_ids: string[] };

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

function summary(video: (typeof VIDEOS)[number]) {
	return {
		id: video.id,
		title: `${video.channel.name} video ${video.id}`,
		thumbnails: [],
		channel: { ...video.channel, avatar: [], verified: false },
		duration_seconds: 600,
		view_count: 1200,
		published_at: '2026-11-20T10:00:00Z',
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false
	};
}

/** An in-memory backend for groups that filters the feed like the real one. */
async function mockApi(page: Page) {
	const state = {
		groups: [
			{ id: WORKSHOP, name: 'Workshop', show_on_home: true, channel_ids: [FORGE.id] },
			{ id: TRAVEL, name: 'Travel', show_on_home: false, channel_ids: [NORTH.id, TIDE.id] }
		] as Group[],
		calls: [] as string[],
		feedQueries: [] as string[]
	};
	let created = 0;

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
	await page.route('**/api/v0/auth/me', (route) => json(route, account));
	await page.route('**/api/v0/settings', (route) =>
		json(route, {
			video_codecs: [],
			max_quality: null,
			default_quality: null,
			home_sections: ['subscriptions'],
			audio_language: null,
			feed_hide_watched: false,
			feed_hide_shorts: false
		})
	);
	await page.route('**/api/v0/progress?*', (route) => json(route, []));
	await page.route('**/api/v0/subscriptions', (route) =>
		json(
			route,
			CHANNELS.map((channel) => ({
				...channel,
				handle: null,
				avatar: [],
				subscriber_count: 1000,
				verified: false,
				subscribed_at: '2026-11-20T10:00:00Z'
			}))
		)
	);
	await page.route('**/api/v0/feed?*', (route) => {
		const params = new URL(route.request().url()).searchParams;
		state.feedQueries.push(params.toString());
		if (params.get('kind') === 'shorts') return json(route, { items: [], continuation: null });
		const group = params.get('group');
		const members = group ? state.groups.find((g) => g.id === group)?.channel_ids : undefined;
		if (group && !members) return json(route, { code: 'not_found', message: '' }, 404);
		const items = VIDEOS.filter((v) => !members || members.includes(v.channel.id)).map(summary);
		return json(route, { items, continuation: null });
	});
	await page.route('**/api/v0/subscription-groups', (route) => {
		const request = route.request();
		if (request.method() === 'POST') {
			const body = request.postDataJSON() as { name: string; channel_ids: string[] };
			state.calls.push(`CREATE ${body.name} ${body.channel_ids.join(',')}`);
			const group = {
				id: `01a2204e-0000-7000-8000-00000000010${created++}`,
				name: body.name,
				show_on_home: false,
				channel_ids: body.channel_ids
			};
			state.groups.push(group);
			return json(route, group, 201);
		}
		return json(route, state.groups);
	});
	await page.route('**/api/v0/subscription-groups/*', (route) => {
		const request = route.request();
		const id = new URL(request.url()).pathname.split('/').pop() ?? '';
		const group = state.groups.find((g) => g.id === id);
		if (!group) return json(route, { code: 'not_found', message: '' }, 404);
		if (request.method() === 'DELETE') {
			state.calls.push(`DELETE ${group.name}`);
			state.groups = state.groups.filter((g) => g !== group);
			return route.fulfill({ status: 204 });
		}
		const body = request.postDataJSON() as { name?: string; show_on_home?: boolean };
		state.calls.push(`PATCH ${group.name} ${JSON.stringify(body)}`);
		Object.assign(group, body);
		return json(route, group);
	});
	await page.route('**/api/v0/subscription-groups/*/position', (route) => {
		const id = new URL(route.request().url()).pathname.split('/').at(-2) ?? '';
		const { index } = route.request().postDataJSON() as { index: number };
		const group = state.groups.find((g) => g.id === id);
		if (!group) return json(route, { code: 'not_found', message: '' }, 404);
		state.calls.push(`MOVE ${group.name} ${index}`);
		state.groups = state.groups.filter((g) => g !== group);
		state.groups.splice(index, 0, group);
		return route.fulfill({ status: 204 });
	});
	await page.route('**/api/v0/subscription-groups/*/channels/*', (route) => {
		const request = route.request();
		const parts = new URL(request.url()).pathname.split('/');
		const channel = parts.at(-1) ?? '';
		const group = state.groups.find((g) => g.id === parts.at(-3));
		if (!group) return json(route, { code: 'not_found', message: '' }, 404);
		const add = request.method() === 'PUT';
		state.calls.push(`${add ? 'ADD' : 'REMOVE'} ${group.name} ${channel}`);
		group.channel_ids = group.channel_ids.filter((c) => c !== channel);
		if (add) group.channel_ids.push(channel);
		return route.fulfill({ status: 204 });
	});
	return state;
}

test('group tabs narrow the feed on the server', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/subscriptions');

	const grid = page.getByRole('list', { name: 'Subscriptions' });
	const tabs = page.getByRole('navigation', { name: 'Groups' });
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
	await expect(tabs.getByRole('link')).toHaveText(['All', 'Workshop', 'Travel']);
	await expect(tabs.getByRole('link', { name: 'All' })).toHaveAttribute('aria-current', 'page');

	await tabs.getByRole('link', { name: 'Travel' }).click();
	await expect(page).toHaveURL(new RegExp(`group=${TRAVEL}$`));
	await expect(tabs.getByRole('link', { name: 'Travel' })).toHaveAttribute('aria-current', 'page');
	await expect(grid.getByRole('heading', { level: 3 })).toHaveText([/north000001/, /tide0000001/]);
	await expect
		.poll(() => state.feedQueries)
		.toContainEqual(expect.stringContaining(`group=${TRAVEL}`));

	// The channel filter only offers the group's channels.
	await page.getByTestId('feed-filter-channel').click();
	await expect(page.getByTestId('feed-filter-channels').getByRole('button')).toHaveText([
		/All channels/,
		new RegExp(NORTH.name),
		new RegExp(TIDE.name)
	]);
	await page.keyboard.press('Escape');

	await tabs.getByRole('link', { name: 'All' }).click();
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
});

test('a deleted group in the link falls back to all channels', async ({ page }) => {
	await mockApi(page);
	await page.goto('/feed/subscriptions?group=01a2204e-0000-7000-8000-0000000000ff');
	const grid = page.getByRole('list', { name: 'Subscriptions' });
	await expect(grid.getByRole('heading', { level: 3 })).toHaveCount(3);
	await expect(
		page.getByRole('navigation', { name: 'Groups' }).getByRole('link', { name: 'All' })
	).toHaveAttribute('aria-current', 'page');
});

test('channels are assigned to groups from the channel list', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/channels');

	const picker = page.getByRole('button', { name: `Groups for ${NORTH.name}` });
	await picker.click();
	const list = page.getByTestId('group-picker-list');
	await expect(list.getByRole('checkbox', { name: 'Travel' })).toBeChecked();
	await list.getByRole('checkbox', { name: 'Workshop' }).click();
	await expect.poll(() => state.calls).toEqual([`ADD Workshop ${NORTH.id}`]);
	await expect(picker).toHaveText('2');
	await list.getByRole('checkbox', { name: 'Travel' }).click();
	await expect.poll(() => state.calls).toContain(`REMOVE Travel ${NORTH.id}`);

	// Names are unique, ignoring case.
	const input = page.getByRole('dialog').getByTestId('group-name-input');
	await input.fill('travel');
	await expect(page.getByRole('dialog').getByRole('alert')).toHaveText('Name already in use');
	await input.fill('Coast');
	await input.press('Enter');
	await expect.poll(() => state.calls).toContain(`CREATE Coast ${NORTH.id}`);
	await expect(list.getByRole('checkbox', { name: 'Coast' })).toBeChecked();
	await page.keyboard.press('Escape');

	const tabs = page.getByRole('navigation', { name: 'Groups' });
	await tabs.getByRole('link', { name: 'Coast' }).click();
	await expect(
		page.getByRole('list', { name: 'Subscribed channels' }).getByRole('listitem')
	).toHaveCount(1);
});

test('groups are renamed, reordered, shown on home and deleted', async ({ page }) => {
	const state = await mockApi(page);
	await page.goto('/feed/channels');

	await page.getByTestId('groups-manage').click();
	const dialog = page.getByRole('dialog', { name: 'Groups' });
	const rows = dialog.getByTestId('group-manager-row');
	await expect(rows).toHaveCount(2);

	await rows.nth(1).getByTestId('group-show-on-home').click();
	await expect.poll(() => state.calls).toEqual(['PATCH Travel {"show_on_home":true}']);

	await dialog.getByRole('button', { name: 'Actions for Travel' }).click();
	await page.getByRole('menuitem', { name: 'Move up' }).click();
	await expect.poll(() => state.calls).toContain('MOVE Travel 0');
	await expect(rows.first()).toContainText('Travel');

	await dialog.getByRole('button', { name: 'Actions for Workshop' }).click();
	await page.getByRole('menuitem', { name: 'Rename' }).click();
	const name = dialog.getByTestId('group-name-input').first();
	await name.fill('Forge');
	await name.press('Enter');
	await expect.poll(() => state.calls).toContain('PATCH Workshop {"name":"Forge"}');
	await expect(rows.nth(1)).toContainText('Forge');

	await dialog.getByRole('button', { name: 'Actions for Forge' }).click();
	await page.getByRole('menuitem', { name: 'Delete' }).click();
	await page.getByTestId('group-delete-confirm').click();
	await expect.poll(() => state.calls).toContain('DELETE Forge');
	await expect(rows).toHaveCount(1);

	await page.keyboard.press('Escape');
	await page.goto('/');
	await expect(page.getByRole('heading', { level: 2, name: 'Travel' })).toBeVisible();
	await expect(page.getByRole('heading', { level: 2, name: 'Forge' })).toHaveCount(0);
});

test('groups shown on home get their own row', async ({ page }) => {
	await mockApi(page);
	await page.goto('/');
	const row = page.getByRole('region', { name: 'Workshop' });
	await expect(row.getByRole('heading', { level: 3 })).toHaveText([/forge000001/]);
	await expect(page.getByRole('heading', { level: 2, name: 'Travel' })).toHaveCount(0);
	await expect(row.getByRole('link', { name: 'Show all' })).toHaveAttribute(
		'href',
		`/feed/subscriptions?group=${WORKSHOP}`
	);
});
