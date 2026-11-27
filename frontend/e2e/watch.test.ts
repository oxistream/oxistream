import { expect, test, type Page } from '@playwright/test';

test.use({ locale: 'en-US' });

const ID = 'k3Rv9fTq2Lw';

const related = Array.from({ length: 6 }, (_, i) => ({
	id: `rel${String(i).padStart(8, '0')}`,
	title: `Related video ${i + 1}`,
	thumbnails: [],
	channel: i % 2 ? null : { id: 'UCrelated', name: 'Bench Notes', avatar: [], verified: false },
	duration_seconds: 300 + i,
	view_count: i % 2 ? null : 1200,
	published_at: null,
	published_text: null,
	is_live: false,
	is_short: false,
	is_upcoming: false
}));

const video = {
	id: ID,
	title: 'Forging a bottle opener from an old horseshoe',
	description: [
		'A horseshoe from the scrap bin becomes a bottle opener.',
		'',
		'0:00 Picking a shoe',
		'1:42 Drawing out the handle',
		'6:18 Punching the opener',
		'14:05 Filing and finish',
		'',
		'Part one: https://youtu.be/dQw4w9WgXcQ',
		'Hardy build: https://example.org/hardy'
	].join('\n'),
	channel: {
		id: 'UCforge',
		name: 'Hammer & Tongs',
		avatar: [],
		subscriber_count: 284000,
		verified: true
	},
	view_count: 482310,
	like_count: 18420,
	published_at: '2026-11-21T09:30:00Z',
	published_text: 'Nov 21, 2026',
	duration_seconds: 1394,
	is_live: false,
	keywords: ['hidden keyword'],
	thumbnails: [],
	subtitles: [
		{
			id: 'en',
			lang: 'en',
			label: 'English',
			auto_generated: false,
			url: `/proxy/subtitle/${ID}/en`
		}
	],
	related
};

const MPD = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011" type="static" mediaPresentationDuration="PT1394S" minBufferTime="PT1.5S"><Period></Period></MPD>`;

type MockResponse = { status: number; body: unknown };

async function mockApi(
	page: Page,
	{
		details = { status: 200, body: video },
		manifest = { status: 200, body: MPD }
	}: { details?: MockResponse; manifest?: MockResponse } = {}
) {
	// Nothing in these tests may reach the backend or YouTube.
	await page.route('**/proxy/**', (route) => route.fulfill({ status: 404, body: '' }));
	await page.route(`**/api/v0/videos/*/manifest.mpd`, (route) =>
		manifest.status === 200
			? route.fulfill({
					status: 200,
					contentType: 'application/dash+xml',
					body: String(manifest.body)
				})
			: route.fulfill({ status: manifest.status, json: manifest.body })
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) =>
		route.fulfill({ status: details.status, json: details.body })
	);
}

test('watch page renders details, player and related videos', async ({ page }) => {
	await mockApi(page);
	await page.goto(`/watch?v=${ID}`);

	await expect(page.getByRole('heading', { level: 1, name: video.title })).toBeVisible();
	await expect(page).toHaveTitle(`${video.title} · Oxistream`);
	await expect(page.locator('video-player')).toBeAttached();
	await expect(page.locator('dash-video > track[kind="subtitles"]')).toHaveAttribute(
		'srclang',
		'en'
	);

	await expect(page.getByRole('link', { name: 'Hammer & Tongs' })).toBeVisible();
	await expect(page.getByText('284K subscribers')).toBeVisible();
	await expect(page.getByTestId('like-button')).toHaveCount(0);
	await expect(page.getByTestId('subscribe-button')).toHaveCount(0);
	await expect(page.getByText('hidden keyword')).toHaveCount(0);

	const relatedList = page.getByRole('region', { name: 'Related videos' });
	await expect(relatedList.getByRole('listitem')).toHaveCount(related.length);
	await expect(relatedList.getByRole('link', { name: /Related video 1/ })).toHaveAttribute(
		'href',
		`/watch?v=${related[0].id}`
	);
});

test('subtitle tracks sharing an id do not break the player', async ({ page }) => {
	const errors: Error[] = [];
	page.on('pageerror', (error) => errors.push(error));
	const english = (label: string) => ({
		id: 'en',
		lang: 'en',
		label,
		auto_generated: false,
		url: `/proxy/subtitle/${ID}/en`
	});
	await mockApi(page, {
		details: {
			status: 200,
			body: { ...video, subtitles: [english('English - en'), english('English - English')] }
		}
	});
	await page.goto(`/watch?v=${ID}`);

	await expect(page.getByRole('heading', { level: 1, name: video.title })).toBeVisible();
	const tracks = page.locator('dash-video > track[kind="subtitles"]');
	await expect(tracks).toHaveCount(2);
	await expect(tracks.nth(1)).toHaveAttribute('label', 'English - English');
	expect(errors.filter((e) => /each_key_duplicate/.test(e.message))).toEqual([]);
});

test('altered or synthetic content is labelled', async ({ page }) => {
	await mockApi(page, {
		details: {
			status: 200,
			body: {
				...video,
				content_disclosures: [
					{
						kind: 'auto_dubbed',
						title: 'Auto-dubbed',
						text: 'Audio tracks for some languages were automatically generated.'
					},
					{
						kind: 'altered_or_synthetic',
						title: 'Made with AI',
						text: 'Sounds or visuals were altered or fully generated.',
						help_url: 'https://support.google.com/youtube/answer/15447836'
					}
				]
			}
		}
	});
	await page.goto(`/watch?v=${ID}`);

	const label = page.getByRole('button', { name: 'Altered or synthetic content' });
	await expect(label).toHaveAttribute('aria-expanded', 'false');
	await expect(page.getByText('Sounds or visuals were altered')).toBeHidden();
	await label.click();
	await expect(page.getByText('Sounds or visuals were altered')).toBeVisible();
	await expect(page.getByRole('link', { name: 'Learn more on YouTube' })).toHaveAttribute(
		'rel',
		'noopener noreferrer'
	);
	await expect(page.getByText('Audio tracks for some languages')).toHaveCount(0);
});

test('description expands and links timestamps and URLs', async ({ page }) => {
	await mockApi(page);
	await page.setViewportSize({ width: 900, height: 900 });
	await page.goto(`/watch?v=${ID}`);

	const description = page.getByTestId('video-description');
	await expect(description).toHaveAttribute('data-expanded', 'false');

	const toggle = page.getByRole('button', { name: 'Show more' });
	await toggle.click();
	await expect(description).toHaveAttribute('data-expanded', 'true');
	await expect(page.getByRole('button', { name: 'Show less' })).toHaveAttribute(
		'aria-expanded',
		'true'
	);

	await expect(description.getByRole('button', { name: 'Jump to 1:42' })).toBeVisible();
	await expect(
		description.getByRole('link', { name: 'https://youtu.be/dQw4w9WgXcQ' })
	).toHaveAttribute('href', '/watch?v=dQw4w9WgXcQ');
	const external = description.getByRole('link', { name: 'https://example.org/hardy' });
	await expect(external).toHaveAttribute('target', '_blank');
	await expect(external).toHaveAttribute('rel', /noopener/);

	await page.getByRole('button', { name: 'Show less' }).click();
	await expect(description).toHaveAttribute('data-expanded', 'false');
});

test('the description is shown in full without a toggle beside the related column', async ({
	page
}) => {
	await mockApi(page);
	await page.setViewportSize({ width: 1700, height: 900 });
	await page.goto(`/watch?v=${ID}`);

	const description = page.getByTestId('video-description');
	await expect(description.getByRole('button', { name: 'Jump to 14:05' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Show more' })).toBeHidden();
	await expect(description).toHaveCSS('-webkit-line-clamp', 'none');

	await page.setViewportSize({ width: 900, height: 900 });
	await expect(page.getByRole('button', { name: 'Show more' })).toBeVisible();
	await expect(description).not.toHaveCSS('-webkit-line-clamp', 'none');
});

test('livestreams show a not-supported state instead of the player', async ({ page }) => {
	await mockApi(page, {
		details: { status: 200, body: { ...video, is_live: true } },
		manifest: {
			status: 404,
			body: { code: 'unavailable', message: 'live', reason: 'livestream' }
		}
	});
	await page.goto(`/watch?v=${ID}`);

	await expect(page.getByTestId('player-message')).toHaveAttribute('data-kind', 'live');
	await expect(page.getByText('Livestreams are not supported yet')).toBeVisible();
	await expect(page.locator('video-player')).toHaveCount(0);
	await expect(page.getByRole('heading', { level: 1, name: video.title })).toBeVisible();
});

test('unavailable videos explain why', async ({ page }) => {
	await mockApi(page, {
		details: {
			status: 404,
			body: { code: 'unavailable', message: 'nope', reason: 'age-restricted' }
		}
	});
	await page.goto(`/watch?v=${ID}`);

	await expect(page.getByText('This video is age-restricted')).toBeVisible();
	await expect(page.getByRole('link', { name: 'Open on YouTube' })).toHaveAttribute(
		'href',
		`https://www.youtube.com/watch?v=${ID}`
	);
	await expect(page).toHaveTitle('This video is age-restricted · Oxistream');
});

test('a watch link without a video id shows an empty state', async ({ page }) => {
	await page.goto('/watch');
	await expect(page.getByRole('heading', { name: 'No video selected' })).toBeVisible();
});
