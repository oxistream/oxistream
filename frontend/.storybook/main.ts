import type { StorybookConfig } from '@storybook/sveltekit';
import type { Plugin } from 'vite';

// SvelteKit defines `__SVELTEKIT_PAYLOAD__` for client builds in its compile plugin, which
// @storybook/sveltekit removes, so the real `$app/paths` would throw in the built Storybook.
// `undefined` matches what SvelteKit itself sets for the default `split` bundle strategy.
const sveltekitBuildDefines: Plugin = {
	name: 'oxistream:storybook-sveltekit-defines',
	apply: 'build',
	config: () => ({ define: { __SVELTEKIT_PAYLOAD__: 'undefined' } })
};

const config: StorybookConfig = {
	stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|ts|svelte)'],
	addons: ['@storybook/addon-svelte-csf', '@storybook/addon-a11y', '@storybook/addon-docs'],
	framework: '@storybook/sveltekit',
	viteFinal: (config) => ({
		...config,
		plugins: [...(config.plugins ?? []), sveltekitBuildDefines]
	})
};

export default config;
