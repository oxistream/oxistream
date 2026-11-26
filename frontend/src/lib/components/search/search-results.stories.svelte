<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import SearchXIcon from '@lucide/svelte/icons/search-x';
	import type { SearchResult } from '#lib/api/search.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { mockChannels, mockVideos } from '#lib/mocks/videos.js';
	import { m } from '#lib/paraglide/messages.js';
	import ChannelResult from './channel-result.svelte';
	import PlaylistResult from './playlist-result.svelte';
	import ResultSkeleton from './result-skeleton.svelte';
	import SearchResults from './search-results.svelte';
	import VideoResult from './video-result.svelte';

	const now = new Date('2026-11-26T12:00:00Z');

	const video = {
		...mockVideos[4],
		channel: { ...mockVideos[4].channel, verified: true },
		description:
			'Two roasts of the same Ethiopian lot side by side: what changes in the cup when development time goes from 90 to 150 seconds, and how to taste it.'
	};

	const channel = {
		...mockChannels.forge,
		verified: true,
		subscriberCount: 284_000,
		description:
			'Traditional blacksmithing from a one-person forge: tools, knives and architectural ironwork, start to finish.'
	};

	const playlist = {
		id: 'PLm0ckFr0mTh3F0rg3',
		title: 'Forge basics: from first fire to finished hook',
		thumbnailUrl: mockVideos[0].thumbnailUrl,
		channel: { ...mockChannels.forge, verified: true },
		videoCount: 24
	};

	const results: SearchResult[] = [
		{ type: 'video', key: 'v:1', video },
		{ type: 'channel', key: 'c:1', channel },
		{
			type: 'video',
			key: 'v:2',
			video: { ...mockVideos[7], isLive: true, durationSeconds: undefined }
		},
		{ type: 'playlist', key: 'p:1', playlist },
		{ type: 'video', key: 'v:3', video: mockVideos[2] }
	];

	const { Story } = defineMeta({
		title: 'Search/Results',
		component: SearchResults,
		parameters: { layout: 'fullscreen' }
	});
</script>

<Story name="Mixed results" asChild>
	<div class="max-w-5xl p-6"><SearchResults {results} {now} /></div>
</Story>

<Story name="Narrow container" asChild>
	<div class="max-w-[390px] p-4"><SearchResults {results} {now} /></div>
</Story>

<Story name="Video row" asChild>
	<div class="@container max-w-5xl p-6"><VideoResult {video} {now} /></div>
</Story>

<Story name="Video row without metadata" asChild>
	<div class="@container max-w-5xl p-6">
		<VideoResult
			video={{
				id: 'dQw4w9WgXcQ',
				title: 'A video whose listing carries only a title',
				channel: { id: '', name: '' }
			}}
		/>
	</div>
</Story>

<Story name="Channel row" asChild>
	<div class="@container max-w-5xl p-6"><ChannelResult {channel} /></div>
</Story>

<Story name="Channel row without image" asChild>
	<div class="@container max-w-5xl p-6">
		<ChannelResult channel={{ ...mockChannels.bindery, verified: false, description: '' }} />
	</div>
</Story>

<Story name="Playlist row" asChild>
	<div class="@container max-w-5xl p-6"><PlaylistResult {playlist} /></div>
</Story>

<Story name="Loading" asChild>
	<div class="max-w-5xl p-6"><SearchResults loading skeletonCount={3} /></div>
</Story>

<Story name="Skeleton row" asChild>
	<div class="@container max-w-5xl p-6"><ResultSkeleton /></div>
</Story>

<Story name="Load more" asChild>
	<div class="max-w-5xl p-6">
		<InfiniteLoader hasMore loading={false} onload={() => {}} />
		<InfiniteLoader hasMore loading onload={() => {}} />
		<InfiniteLoader hasMore loading={false} failed onload={() => {}} />
		<InfiniteLoader hasMore={false} loading={false} onload={() => {}} endLabel={m.search_end()} />
	</div>
</Story>

<Story name="No results" asChild>
	<div class="max-w-5xl p-6">
		<EmptyState
			icon={SearchXIcon}
			title={m.search_empty_title({ query: 'qwxz forge' })}
			description={m.search_empty_body()}
		/>
	</div>
</Story>

<Story name="Error" asChild>
	<div class="max-w-5xl p-6">
		<ErrorState
			title={m.search_error_title()}
			description={m.search_error_body()}
			onretry={() => {}}
		/>
	</div>
</Story>
