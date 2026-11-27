import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	testDir: 'e2e',
	webServer: {
		command: 'pnpm build && pnpm preview --port 4173 --strictPort',
		port: 4173,
		reuseExistingServer: !process.env.CI
	},
	use: { baseURL: 'http://localhost:4173' },
	projects: [
		{ name: 'chromium', use: { ...devices['Desktop Chrome'] } },
		// WebKit plays media differently enough that room sync is checked there too.
		{
			name: 'webkit',
			use: { ...devices['Desktop Safari'] },
			testMatch: 'together.test.ts',
			grep: /few frames|scrubbing/
		}
	]
});
