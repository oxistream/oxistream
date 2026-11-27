import { expect, test, type Locator, type Page, type Route } from '@playwright/test';
import type { TrendingCategory } from '../src/lib/api/trending';

test.use({ locale: 'en-US' });

const PIXEL = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
	'base64'
);

const account = {
	id: '0193a8e4-7f00-7000-8000-000000000001',
	username: 'mara',
	role: 'user',
	has_password: true,
	created_at: '2026-11-26T10:00:00Z'
};

const channel = {
	id: 'UCm0ck4nv1lF0rg3H4mm3r1',
	name: 'Hammer & Tongs',
	avatar: [],
	verified: true
};

function video(prefix: string, n: number) {
	return {
		id: `${prefix}${String(n).padStart(11 - prefix.length, '0')}`,
		title: `${prefix} video ${n}`,
		thumbnails: [{ url: `/proxy/image/${prefix}${n}`, width: 480, height: 270 }],
		duration_seconds: 300,
		view_count: 1000,
		published_at: '2026-11-20T10:00:00Z',
		published_text: null,
		is_live: false,
		is_short: false,
		is_upcoming: false,
		channel
	};
}

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

type Trending = Partial<Record<TrendingCategory, number | 'error'>>;

async function mockApi(
	page: Page,
	{
		signedIn = false,
		trending = {},
		subscriptions = 0,
		homeSections = null
	}: {
		signedIn?: boolean;
		trending?: Trending;
		subscriptions?: number;
		homeSections?: string[] | null;
	} = {}
) {
	await page.route('**/proxy/image/**', (route) =>
		route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL })
	);
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
	await page.route('**/api/v0/trending?*', (route) => {
		const category = new URL(route.request().url()).searchParams.get('category') as
			keyof Trending | null;
		const count = category ? trending[category] : undefined;
		if (count === undefined || count === 'error') {
			return json(route, { code: 'upstream', message: '' }, 502);
		}
		return json(route, {
			category,
			region: 'US',
			items: Array.from({ length: count }, (_, i) => video(category ?? 'x', i + 1))
		});
	});
	await page.route('**/api/v0/subscriptions', (route) =>
		json(
			route,
			Array.from({ length: subscriptions }, () => ({
				...channel,
				handle: null,
				subscriber_count: null,
				subscribed_at: '2026-11-20T10:00:00Z'
			}))
		)
	);
	await page.route('**/api/v0/feed?*', (route) =>
		json(route, {
			items: Array.from({ length: 20 }, (_, i) => video('feed', i + 1)),
			continuation: null
		})
	);
	await page.route('**/api/v0/settings', (route) =>
		json(route, {
			video_codecs: [],
			max_quality: null,
			default_quality: null,
			home_sections: homeSections
		})
	);
	await page.route('**/api/v0/history/continue?*', (route) => json(route, []));
	await page.route('**/api/v0/progress?*', (route) => json(route, []));
}

test('guests see trending rows and empty or failing ones are hidden', async ({ page }) => {
	await mockApi(page, { trending: { news: 3, gaming: 0, podcasts: 'error' } });
	await page.goto('/');

	const news = page.getByRole('list', { name: 'News' });
	await expect(news.getByRole('listitem')).toHaveCount(3);
	await expect(page.getByRole('heading', { level: 2 })).toHaveText(['News']);
	await expect(page.getByRole('heading', { name: 'What do you want to watch?' })).toHaveCount(0);
});

test('rows hide their scrollbar and page with arrow buttons', async ({ page }) => {
	await mockApi(page, { trending: { news: 12 } });
	await page.goto('/');

	const news = page.getByRole('list', { name: 'News' });
	await expect(news.getByRole('listitem')).toHaveCount(12);
	await expect(news).toHaveCSS('scrollbar-width', 'none');

	const region = page.getByRole('region', { name: 'News' });
	const previous = region.getByRole('button', { name: 'Previous' });
	const next = region.getByRole('button', { name: 'Next' });
	await expect(previous).toBeHidden();
	await expect(next).toBeVisible();

	const firstOffscreen = news.getByRole('listitem').nth(await visibleCount(news));
	await next.click();
	await expect(previous).toBeVisible();
	await expect(firstOffscreen).toBeInViewport({ ratio: 1 });

	await next.focus();
	await news.evaluate((el) => el.scrollTo({ left: el.scrollWidth }));
	await expect(next).toBeHidden();
	await expect(previous).toBeFocused();
	await previous.click();
	await expect(next).toBeVisible();
});

async function visibleCount(list: Locator) {
	return list.evaluate((el) => {
		const end = el.getBoundingClientRect().right;
		return [...el.children].filter((item) => item.getBoundingClientRect().right <= end).length;
	});
}

test('rows follow the default order', async ({ page }) => {
	await mockApi(page, { trending: { news: 1, gaming: 2, podcasts: 1 } });
	await page.goto('/');
	await expect(page.getByRole('heading', { level: 2 })).toHaveText(['News', 'Gaming', 'Podcasts']);
});

test('guests get a search box and their local history when trending fails', async ({ page }) => {
	await page.addInitScript(() => {
		const entry = (title: string, updatedAt: number) => ({
			position: 60,
			duration: 120,
			finished: false,
			updatedAt,
			video: { title, channel: { id: 'UCm0ck4nv1lF0rg3H4mm3r1', name: 'Hammer & Tongs' } }
		});
		localStorage.setItem(
			'oxistream:progress',
			JSON.stringify({
				lcl00000001: entry('Older local video', 1),
				lcl00000002: entry('Newer local video', 2),
				lcl00000003: { position: 1, duration: 2, finished: false, updatedAt: 3 }
			})
		);
	});
	await mockApi(page);
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'What do you want to watch?' })).toBeVisible();
	const recent = page.getByRole('list', { name: 'Recently watched' });
	await expect(recent.getByRole('listitem')).toHaveCount(2);
	await expect(recent.getByRole('link').first()).toHaveAccessibleName(
		/Newer local video.*50% watched/
	);

	await page.getByTestId('search-input-large').fill('forging');
	await page.getByTestId('search-input-large').press('Enter');
	await expect(page).toHaveURL(/\/results\?search_query=forging$/);
});

test('signed-in users see the latest videos of their subscriptions', async ({ page }) => {
	await mockApi(page, { signedIn: true, subscriptions: 1, trending: { news: 3 } });
	await page.goto('/');

	const latest = page.getByRole('region', { name: 'Latest from your subscriptions' });
	await expect(latest.getByRole('listitem')).toHaveCount(15);
	await expect(page.getByRole('heading', { name: 'News' })).toHaveCount(0);
	await latest.getByRole('link', { name: 'Show all' }).click();
	await expect(page).toHaveURL(/\/feed\/subscriptions$/);
});

test('signed-in users without subscriptions are shown where to get some', async ({ page }) => {
	await mockApi(page, { signedIn: true });
	await page.goto('/');

	const latest = page.getByRole('region', { name: 'Latest from your subscriptions' });
	await expect(latest.getByRole('heading', { name: 'No subscriptions yet' })).toBeVisible();
	await expect(latest.getByRole('link', { name: 'Find channels' })).toHaveAttribute(
		'href',
		'/results'
	);
	await expect(latest.getByRole('link', { name: 'Import / export' })).toHaveAttribute(
		'href',
		'/subscriptions/import-export'
	);
});

test('signed-in users get the rows they chose, in their order', async ({ page }) => {
	await mockApi(page, {
		signedIn: true,
		subscriptions: 1,
		trending: { news: 2, podcasts: 2, fashion: 2 },
		homeSections: [
			'trending-podcasts',
			'subscriptions',
			'trending-fashion',
			'trending-news',
			'removed-section'
		]
	});
	await page.goto('/');
	await expect(page.getByRole('heading', { level: 2 })).toHaveText([
		'Podcasts',
		'Latest from your subscriptions',
		'Fashion & Beauty',
		'News'
	]);
});

test('an empty home page points to the settings', async ({ page }) => {
	await mockApi(page, { signedIn: true, homeSections: [] });
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'Your home page is empty' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Open settings' })).toHaveAttribute(
		'href',
		'/settings'
	);
});
