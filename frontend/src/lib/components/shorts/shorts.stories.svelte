<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockChannels, mockVideos } from '#lib/mocks/videos.js';
	import ShortOverlay from './short-overlay.svelte';
	import ShortRow from './short-row.svelte';

	const { Story } = defineMeta({
		title: 'Shorts/ShortOverlay',
		component: ShortOverlay,
		parameters: { layout: 'centered' },
		args: {
			videoId: 'dQw4w9WgXcQ',
			title: 'Forging a chef knife from an old file in under a minute',
			channel: mockChannels.forge
		}
	});

	const shorts = mockVideos.map((video, i) => ({
		...video,
		durationSeconds: undefined,
		progress: i === 1 ? 0.4 : undefined
	}));
</script>

{#snippet template(args: Parameters<typeof ShortOverlay>[1])}
	<div class="relative aspect-9/16 h-[34rem] overflow-hidden rounded-xl bg-[oklch(0.13_0.004_286)]">
		<img src={mockVideos[0].thumbnailUrl} alt="" class="size-full object-cover" />
		<ShortOverlay {...args} />
	</div>
{/snippet}

<Story name="Overlay" {template} />

<Story name="Overlay without channel" args={{ channel: { id: '', name: '' } }} {template} />

<Story name="Shelf" parameters={{ layout: 'fullscreen' }} asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<ShortRow {shorts} label="Shorts" />
	</div>
</Story>

<Story name="Shelf loading" parameters={{ layout: 'fullscreen' }} asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<ShortRow loading />
	</div>
</Story>
