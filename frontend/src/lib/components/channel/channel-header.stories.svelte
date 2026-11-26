<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import banner from '#lib/mocks/thumbnails/westfjords-road.jpg';
	import { mockChannels } from '#lib/mocks/videos.js';
	import ChannelHeaderSkeleton from './channel-header-skeleton.svelte';
	import ChannelHeader, { type ChannelHeaderData } from './channel-header.svelte';

	const channel: ChannelHeaderData = {
		...mockChannels.north,
		avatarLargeUrl: mockChannels.north.avatarUrl,
		bannerUrl: banner,
		verified: true,
		subscriberCount: 1_240_000,
		videoCount: 312,
		description:
			'Slow road trips through the North Atlantic: Faroe Islands, Iceland and the Westfjords, filmed on one camera with natural sound.\n\nNew episodes every other Sunday. Routes, maps and gear lists are in each description.'
	};

	const { Story } = defineMeta({
		title: 'Channel/ChannelHeader',
		component: ChannelHeader,
		args: { channel },
		parameters: { layout: 'fullscreen' }
	});
</script>

{#snippet template(args: Parameters<typeof ChannelHeader>[1])}
	<div class="p-6"><ChannelHeader {...args} /></div>
{/snippet}

<Story name="Default" {template} />

<Story
	name="Without banner"
	args={{ channel: { ...channel, bannerUrl: undefined, videoCount: undefined } }}
	{template}
/>

<Story
	name="Without images or description"
	args={{
		channel: {
			...mockChannels.bindery,
			avatarUrl: undefined,
			verified: false,
			subscriberCount: 1,
			description: ''
		}
	}}
	{template}
/>

<Story
	name="Long name"
	args={{
		channel: {
			...channel,
			name: 'Northbound — Slow Travel Films from the Far North Atlantic and Beyond'
		}
	}}
	{template}
/>

<Story name="Phone width" asChild>
	<div class="max-w-[390px] p-4"><ChannelHeader {channel} /></div>
</Story>

<Story name="Skeleton" asChild>
	<div class="p-6"><ChannelHeaderSkeleton /></div>
</Story>
