<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import SegmentOverlay from './segment-overlay.svelte';

	const { Story } = defineMeta({
		title: 'Player/SegmentOverlay',
		component: SegmentOverlay,
		args: { onundo: () => {}, onskip: () => {}, onhighlight: () => {} }
	});
</script>

{#snippet template(args: Parameters<typeof SegmentOverlay>[1])}
	<div
		class="relative aspect-video max-w-3xl overflow-hidden rounded-xl"
		style="background: linear-gradient(135deg, oklch(0.45 0.05 230), oklch(0.25 0.03 260))"
	>
		<SegmentOverlay {...args} />
	</div>
{/snippet}

<Story name="Skip button" args={{ skippable: 'intro' }} {template} />

<Story name="Skipped with undo" args={{ skipped: 'sponsor' }} {template} />

<Story
	name="Skipped while another offers a button"
	args={{ skipped: 'sponsor', skippable: 'selfpromo' }}
	{template}
/>

<Story name="Highlight" args={{ highlight: true }} {template} />

<Story name="Full-video label" args={{ label: 'exclusive_access', highlight: true }} {template} />
