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
		// Safari plays media differently enough that room sync is checked there too. Only on macOS:
		// WebKit on Linux plays through GStreamer and says nothing about Safari.
		...(process.platform === 'darwin'
			? [
					{
						name: 'webkit',
						use: { ...devices['Desktop Safari'] },
						testMatch: 'together.test.ts',
						grep: /few frames|scrubbing/
					}
				]
			: [])
	]
});
