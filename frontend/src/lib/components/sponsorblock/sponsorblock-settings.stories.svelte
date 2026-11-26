<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { defaultSettings, type SponsorBlockSettings } from '#lib/sponsorblock/settings.js';
	import SponsorBlockSettingsForm from './sponsorblock-settings.svelte';

	const { Story } = defineMeta({
		title: 'SponsorBlock/Settings',
		component: SponsorBlockSettingsForm,
		args: { settings: defaultSettings(), onchange: () => {}, signedIn: true }
	});

	const customized: SponsorBlockSettings = {
		...defaultSettings(),
		categories: { ...defaultSettings().categories, intro: 'auto', filler: 'off', hook: 'off' },
		highlight: false
	};
</script>

<script lang="ts">
	let interactive = $state(defaultSettings());
</script>

{#snippet template(args: Parameters<typeof SponsorBlockSettingsForm>[1])}
	<div class="max-w-xl rounded-xl border bg-card p-5">
		<SponsorBlockSettingsForm {...args} />
	</div>
{/snippet}

<Story name="Defaults" {template} />

<Story name="Customized" args={{ settings: customized }} {template} />

<Story name="Guest" args={{ signedIn: false }} {template} />

<Story name="Save failed" args={{ saveFailed: true }} {template} />

<Story name="Interactive">
	<div class="max-w-xl rounded-xl border bg-card p-5">
		<SponsorBlockSettingsForm settings={interactive} onchange={(next) => (interactive = next)} />
	</div>
</Story>
