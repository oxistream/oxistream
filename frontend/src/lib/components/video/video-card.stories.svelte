<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockChannels, mockVideos } from '#lib/mocks/videos.js';
	import VideoCardSkeleton from './video-card-skeleton.svelte';
	import VideoCard from './video-card.svelte';

	const { Story } = defineMeta({
		title: 'Video/VideoCard',
		component: VideoCard,
		args: { video: mockVideos[4] }
	});
</script>

{#snippet template(args: Parameters<typeof VideoCard>[1])}
	<div class="w-80">
		<VideoCard {...args} />
	</div>
{/snippet}

<Story name="Default" {template} />

<Story name="With watch progress" args={{ video: mockVideos[1] }} {template} />

<Story
	name="Long title"
	args={{
		video: {
			...mockVideos[6],
			title:
				'Hand-gilding raised bands on a full leather spine, with every tool, every mistake and the second attempt that finally worked'
		}
	}}
	{template}
/>

<Story
	name="Without images"
	args={{
		video: {
			...mockVideos[6],
			thumbnailUrl: undefined,
			channel: { ...mockChannels.bindery, avatarUrl: undefined }
		}
	}}
	{template}
/>

<Story
	name="Hours long, one view"
	args={{ video: { ...mockVideos[12], viewCount: 1 } }}
	{template}
/>

<Story name="Without channel" args={{ video: mockVideos[0], showChannel: false }} {template} />

<Story name="Skeleton" asChild>
	<div class="w-80"><VideoCardSkeleton /></div>
</Story>
