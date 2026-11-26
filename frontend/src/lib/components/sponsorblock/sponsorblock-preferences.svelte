<script lang="ts">
	import { useSponsorBlockSettings } from '#lib/api/sponsorblock.svelte.js';
	import SponsorBlockSettings from './sponsorblock-settings.svelte';

	let {
		headingLevel = 2,
		header = true,
		class: className
	}: { headingLevel?: 2 | 3; header?: boolean; class?: string } = $props();

	const store = useSponsorBlockSettings();
</script>

<SponsorBlockSettings
	settings={store.current}
	onchange={(next) => store.update(next)}
	signedIn={store.signedIn}
	saveFailed={store.saveFailed}
	{headingLevel}
	{header}
	class={className}
/>
