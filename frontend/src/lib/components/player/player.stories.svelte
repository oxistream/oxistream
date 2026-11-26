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
