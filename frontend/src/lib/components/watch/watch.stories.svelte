<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { ApiError } from '#lib/api/mapping.js';
	import { mockVideos } from '#lib/mocks/videos.js';
	import ChannelRow from './channel-row.svelte';
	import { describeProblem, liveProblem } from './errors';
	import { mockNow, mockRelated, mockVideoDetails } from './mocks';
	import PlayerMessage from './player-message.svelte';
	import RelatedVideos from './related-videos.svelte';
	import VideoDescription from './video-description.svelte';
	import VideoDetailsSkeleton from './video-details-skeleton.svelte';
	import VideoDetails from './video-details.svelte';

	const { Story } = defineMeta({
		title: 'Watch/VideoDetails',
		component: VideoDetails,
		args: { video: mockVideoDetails, now: mockNow }
	});

	const unavailable = (reason: string) =>
		describeProblem(new ApiError(404, { code: 'unavailable', message: '', reason }));
</script>

{#snippet template(args: Parameters<typeof VideoDetails>[1])}
	<div class="max-w-3xl">
		<VideoDetails {...args} />
	</div>
{/snippet}

<Story name="Default" {template} />

<Story name="Description expanded" args={{ descriptionExpanded: true }} {template} />

<Story
	name="Short description, no likes"
	args={{
		video: {
			...mockVideoDetails,
			description: 'Short and sweet.',
			like_count: null,
			channel: { ...mockVideoDetails.channel, verified: false, subscriber_count: 1 }
		}
	}}
	{template}
/>

<Story
	name="Long title, no description"
	args={{
		video: {
			...mockVideoDetails,
			description: '',
			title:
				'Forging a bottle opener from an old horseshoe, start to finish, with every heat, every mistake and the second attempt that finally worked'
		}
	}}
	{template}
/>

<Story name="Skeleton" asChild>
	<div class="max-w-3xl"><VideoDetailsSkeleton /></div>
</Story>

<Story name="Channel row" asChild>
	<div class="flex max-w-md flex-col gap-4">
		<ChannelRow
			channel={{
				id: mockVideos[0].channel.id,
				name: mockVideos[0].channel.name,
				avatarUrl: mockVideos[0].channel.avatarUrl,
				subscriberCount: 284_000,
				verified: true
			}}
		/>
		<ChannelRow channel={{ id: 'UCnone', name: 'Bindery Hours' }} />
	</div>
</Story>

<Story name="Description with chapters" asChild>
	<div class="max-w-2xl">
		<VideoDescription
			description={mockVideoDetails.description}
			viewCount={482_310}
			publishedAt="2026-11-21T09:30:00Z"
			durationSeconds={1394}
			expanded
			now={mockNow}
		/>
	</div>
</Story>

<Story name="Related videos" asChild>
	<div class="w-88"><RelatedVideos videos={mockRelated} now={mockNow} /></div>
</Story>

<Story name="Related videos, wide" asChild>
	<div class="max-w-4xl"><RelatedVideos videos={mockRelated} now={mockNow} /></div>
</Story>

<Story name="Related videos loading" asChild>
	<div class="w-88"><RelatedVideos loading /></div>
</Story>

<Story name="Related videos empty" asChild>
	<div class="w-88"><RelatedVideos /></div>
</Story>

<Story name="Player message: livestream" asChild>
	<div class="max-w-3xl">
		<PlayerMessage
			problem={liveProblem()}
			videoId="k3Rv9fTq2Lw"
			poster={mockVideos[0].thumbnailUrl}
		/>
	</div>
</Story>

<Story name="Player message: age-restricted" asChild>
	<div class="max-w-3xl">
		<PlayerMessage problem={unavailable('age-restricted')} videoId="k3Rv9fTq2Lw" />
	</div>
</Story>

<Story name="Player message: rate limited" asChild>
	<div class="max-w-3xl">
		<PlayerMessage
			problem={describeProblem(new ApiError(503, { code: 'rate_limited', message: '' }))}
			onretry={() => {}}
		/>
	</div>
</Story>

<Story name="Player message: not found" asChild>
	<div class="max-w-3xl">
		<PlayerMessage
			problem={describeProblem(new ApiError(404, { code: 'not_found', message: '' }))}
		/>
	</div>
</Story>
