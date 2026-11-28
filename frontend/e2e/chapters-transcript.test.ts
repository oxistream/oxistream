import { readFileSync } from 'node:fs';
import { expect, test, type Locator, type Page, type Route } from '@playwright/test';

test.use({
	locale: 'en-US',
	launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] }
});

const ID = 'chapterVid1';
const CLIP = readFileSync(new URL('./fixtures/sync.webm', import.meta.url));
const MPD = readFileSync(new URL('./fixtures/sync.mpd', import.meta.url), 'utf8');

// Rolling auto-captions: each cue repeats the line before it.
const VTT = [
	'WEBVTT',
	'Kind: captions',
	'Language: en',
	'',
	'00:00:01.000 --> 00:00:04.000 align:start position:0%',
	' ',
	'light<00:00:01.500><c> the</c><00:00:02.000><c> forge</c>',
	'',
	'00:00:04.000 --> 00:00:04.010 align:start position:0%',
	'light the forge',
	' ',
	'',
	'00:00:04.010 --> 00:00:30.000 align:start position:0%',
	'light the forge',
	'heat<00:00:05.000><c> the</c><00:00:06.000><c> steel</c>',
	'',
	'00:00:30.000 --> 00:00:45.000 align:start position:0%',
	'heat the steel',
	'quench<00:00:31.000><c> the</c><00:00:32.000><c> steel</c>',
	''
].join('\n');

const json = (route: Route, body: unknown, status = 200) =>
	route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });

async function mockApi(page: Page) {
	// Nothing in these tests may reach the backend or YouTube.
	await page.route('**/proxy/**', (route) => route.fulfill({ status: 404, body: '' }));
	await page.route('**/proxy/subtitle/**', (route) =>
		route.fulfill({ status: 200, contentType: 'text/vtt', body: VTT })
	);
	await page.route('**/proxy/stream/**', (route) => {
		const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers()['range'] ?? '');
		if (!range) return route.fulfill({ status: 200, contentType: 'video/webm', body: CLIP });
		const start = Number(range[1]);
		const end = range[2] ? Number(range[2]) : CLIP.length - 1;
		return route.fulfill({
			status: 206,
			contentType: 'video/webm',
			headers: { 'Content-Range': `bytes ${start}-${end}/${CLIP.length}` },
			body: CLIP.subarray(start, end + 1)
		});
	});
	await page.route('**/api/v0/**', (route) => json(route, { code: 'not_found', message: '' }, 404));
	await page.route('**/api/v0/auth/me', (route) =>
		json(route, { code: 'unauthorized', message: '' }, 401)
	);
	await page.route('**/api/v0/videos/*/manifest.mpd', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/dash+xml',
			body: MPD.replace('{id}', ID)
		})
	);
	await page.route(/\/api\/v0\/videos\/[^/]+$/, (route) =>
		json(route, {
			id: ID,
			title: 'Forge day',
			description: '',
			thumbnails: [],
			channel: {
				id: 'UCforge',
				name: 'Forge',
				avatar: [],
				subscriber_count: null,
				verified: false
			},
			view_count: null,
			like_count: null,
			published_at: null,
			published_text: null,
			duration_seconds: 60,
			is_live: false,
			keywords: [],
			subtitles: [
				{
					id: 'a.en',
					lang: 'en',
					label: 'English',
					auto_generated: true,
					url: `/proxy/subtitle/${ID}/a.en`
				}
			],
			audio_tracks: [],
			content_disclosures: [],
			chapters: [
				{ title: 'Lighting up', start_seconds: 0, thumbnails: [] },
				{ title: 'Heating', start_seconds: 20, thumbnails: [] },
				{ title: 'Quenching', start_seconds: 40, thumbnails: [] }
			],
			related: []
		})
	);
}

type Media = HTMLMediaElement;
const time = (media: Locator) => media.evaluate((el) => (el as Media).currentTime);

/** Starts playback, retrying where the browser held back the first play. */
async function playing(media: Locator) {
	await expect
		.poll(
			async () => {
				const paused = await media.evaluate((el) => (el as Media).paused);
				if (paused) await media.evaluate((el) => (el as Media).play().catch(() => {}));
				return time(media);
			},
			{ timeout: 15_000 }
		)
		.toBeGreaterThan(0.5);
}

test.beforeEach(async ({ page }) => {
	// The panel starts open, on the chapters tab.
	await page.addInitScript(() =>
		localStorage.setItem('oxistream:watch-panel', JSON.stringify({ open: true, tab: 'chapters' }))
	);
	await mockApi(page);
});

test('a chapter click seeks there and highlights it', async ({ page }) => {
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await playing(media);

	const chapters = page.getByTestId('chapter');
	await expect(chapters).toHaveCount(3);
	await expect(chapters.first()).toHaveAttribute('aria-current', 'true');

	await chapters.filter({ hasText: 'Quenching' }).click();
	await expect.poll(() => time(media)).toBeGreaterThanOrEqual(40);
	await expect(chapters.filter({ hasText: 'Quenching' })).toHaveAttribute('aria-current', 'true');
	await expect(chapters.first()).not.toHaveAttribute('aria-current');
});

test('the transcript is searchable and a line click seeks there', async ({ page }) => {
	await page.goto(`/watch?v=${ID}`);
	const media = page.locator('dash-video');
	await playing(media);

	await page.getByRole('tab', { name: 'Transcript' }).click();
	const lines = page.getByTestId('transcript-line');
	// Rolled-over repeats are dropped.
	await expect(lines).toHaveText([/light the forge/, /heat the steel/, /quench the steel/]);

	await page.getByTestId('transcript-search').fill('steel');
	await expect(page.getByTestId('transcript-matches')).toHaveText('1/2');
	await expect(lines.nth(1).locator('mark')).toHaveText('steel');
	await page.getByTestId('transcript-next').click();
	await expect(page.getByTestId('transcript-matches')).toHaveText('2/2');
	await expect(lines.nth(2)).toHaveAttribute('data-match', 'active');
	await expect(page.getByTestId('transcript-resume')).toBeVisible();

	await lines.nth(2).click();
	await expect.poll(() => time(media)).toBeGreaterThanOrEqual(30);
	await expect(lines.nth(2)).toHaveAttribute('aria-current', 'true');
	await expect(page.getByTestId('transcript-resume')).toBeHidden();
});
