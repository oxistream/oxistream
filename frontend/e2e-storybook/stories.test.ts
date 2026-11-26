import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

type Entry = { id: string; title: string; name: string; type: 'story' | 'docs' };

const index = JSON.parse(readFileSync('storybook-static/index.json', 'utf8')) as {
	entries: Record<string, Entry>;
};

for (const entry of Object.values(index.entries)) {
	test(`${entry.title} / ${entry.name}`, async ({ page }) => {
		// Stories run offline so the check never depends on third-party hosts (e.g. the player's
		// demo MPD). The resulting "failed to load resource" errors are expected and ignored.
		const blocked = new Set<string>();
		await page.route(/^https?:\/\/(?!localhost[:/])/, (route) => {
			blocked.add(route.request().url());
			return route.abort('blockedbyclient');
		});

		const errors: string[] = [];
		page.on('pageerror', (error) => errors.push(error.message));
		page.on('console', (message) => {
			if (message.type() === 'error' && !blocked.has(message.location().url)) {
				errors.push(message.text());
			}
		});

		const viewMode = entry.type === 'docs' ? 'docs' : 'story';
		await page.goto(`/iframe.html?id=${entry.id}&viewMode=${viewMode}`);
		await expect(page.locator('body')).toHaveClass(/sb-show-(main|errordisplay|nopreview)/);
		await page.waitForLoadState('networkidle');

		expect(errors).toEqual([]);
		await expect(page.locator('body')).toHaveClass(/sb-show-main/);
	});
}
