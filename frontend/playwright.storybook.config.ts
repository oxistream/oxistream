import { defineConfig, devices } from '@playwright/test';

// Set STORYBOOK_URL to check an already running Storybook (e.g. `pnpm storybook`) instead.
const external = process.env.STORYBOOK_URL;

export default defineConfig({
	testDir: 'e2e-storybook',
	outputDir: 'test-results/storybook',
	webServer: external
		? undefined
		: {
				command: 'vite preview -c e2e-storybook/vite.config.ts',
				port: 6007,
				reuseExistingServer: !process.env.CI
			},
	use: { baseURL: external ?? 'http://localhost:6007' },
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }]
});
