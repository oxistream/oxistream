import { defineConfig } from 'vite';

// Plain static server for storybook-static, without the SvelteKit plugin of the app config.
export default defineConfig({
	build: { outDir: 'storybook-static' },
	preview: { port: 6007, strictPort: true }
});
