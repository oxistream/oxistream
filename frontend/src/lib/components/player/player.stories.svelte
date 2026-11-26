<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockVideos } from '#lib/mocks/videos.js';
	import Player from './player.svelte';

	// Storybook has no backend, so the player streams the public DASH test clip Video.js documents.
	const DEMO_MPD = 'https://dash.akamaized.net/akamai/streamroot/050714/Spring_4Ktest.mpd';

	const { Story } = defineMeta({
		title: 'Player/Player',
		component: Player,
		args: {
			src: DEMO_MPD,
			poster: mockVideos[1].thumbnailUrl,
			globalHotkeys: false
		}
	});
</script>

{#snippet template(args: Parameters<typeof Player>[1])}
	<div class="max-w-3xl">
		<Player {...args} />
	</div>
{/snippet}

<Story name="Default" {template} />

<Story name="Start time" args={{ startTime: 30 }} {template} />

<Story name="With theater button" args={{ ontheatertoggle: () => {} }} {template} />

<Story name="Theater" args={{ theater: true, ontheatertoggle: () => {} }} {template} />

<Story
	name="SponsorBlock segments"
	args={{
		segments: [
			{ id: 'i', category: 'intro', kind: 'skip', mode: 'button', start: 0, end: 4 },
			{ id: 's', category: 'sponsor', kind: 'skip', mode: 'auto', start: 8, end: 14 },
			{ id: 'h', category: 'poi_highlight', kind: 'highlight', start: 20, end: 20 },
			{ id: 'f', category: 'filler', kind: 'skip', mode: 'button', start: 30, end: 45 },
			{ id: 'x', category: 'exclusive_access', kind: 'full', start: 0, end: 0 }
		],
		onsponsorblocksettings: () => {}
	}}
	{template}
/>

<Story
	name="SponsorBlock hover labels"
	args={{
		segments: [
			{ id: 's', category: 'sponsor', kind: 'skip', mode: 'auto', start: 10, end: 60 },
			{ id: 'o', category: 'outro', kind: 'skip', mode: 'button', start: 100, end: 134 },
			{ id: 'h', category: 'poi_highlight', kind: 'highlight', start: 75, end: 75 }
		]
	}}
	{template}
/>
