<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import UserXIcon from '@lucide/svelte/icons/user-x';
	import { resolve } from '$app/paths';
	import { createChannelQuery, createChannelVideosQuery } from '#lib/api/channel.js';
	import { isNotFound, uniqueBy } from '#lib/api/mapping.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import ChannelHeaderSkeleton from './channel-header-skeleton.svelte';
	import ChannelHeader from './channel-header.svelte';

	let {
		id,
		/** Set while a "@handle" is still being resolved to an id. */
		resolving = false,
		/** Resolution of the handle failed. */
		resolveError,
		onretry
	}: {
		id?: string;
		resolving?: boolean;
		resolveError?: unknown;
		onretry?: () => void;
	} = $props();

	const channel = createChannelQuery(() => id);
	const videos = createChannelVideosQuery(() => (channel.data ? id : undefined));

	const items = $derived(
		uniqueBy(videos.data?.pages.flatMap((p) => p.items) ?? [], (video) => video.id)
	);
	const error = $derived(resolveError ?? (channel.data ? undefined : channel.error));
	const loading = $derived(!error && (resolving || channel.isPending));
</script>

<svelte:head>
	{#if channel.data}
		<title>{pageTitle(channel.data.name)}</title>
	{:else if error && isNotFound(error)}
		<title>{pageTitle(m.channel_not_found_title())}</title>
	{:else}
		<title>{pageTitle(m.channel_title())}</title>
	{/if}
</svelte:head>

<div class="flex w-full flex-col gap-8 px-(--page-gutter) py-6">
	{#if error}
		<h1 class="sr-only">{m.channel_title()}</h1>
		{#if isNotFound(error)}
			<EmptyState
				icon={UserXIcon}
				title={m.channel_not_found_title()}
				description={m.channel_not_found_body()}
			>
				{#snippet actions()}
					<Button href={resolve('/')} variant="outline">{m.error_back_home()}</Button>
				{/snippet}
			</EmptyState>
		{:else}
			<ErrorState
				title={m.channel_error_title()}
				onretry={() => (onretry && resolveError ? onretry() : void channel.refetch())}
			/>
		{/if}
	{:else if loading || !channel.data}
		<div role="status" aria-label={m.channel_loading()} class="flex flex-col gap-8">
			<ChannelHeaderSkeleton />
			<VideoGrid loading showChannel={false} skeletonCount={8} />
		</div>
	{:else}
		<ChannelHeader channel={channel.data} />
		<section aria-labelledby="channel-videos-heading" class="flex flex-col gap-4">
			<h2 id="channel-videos-heading" class="border-b pb-2 text-base font-semibold tracking-tight">
				{m.channel_videos()}
			</h2>
			{#if videos.isPending}
				<VideoGrid loading showChannel={false} skeletonCount={8} />
			{:else if videos.isError && !videos.data}
				<ErrorState onretry={() => void videos.refetch()} />
			{:else if items.length === 0}
				<EmptyState
					icon={ListVideoIcon}
					headingLevel={3}
					title={m.channel_no_videos_title()}
					description={m.channel_no_videos_body()}
				/>
			{:else}
				<VideoGrid videos={items} showChannel={false} label={m.channel_videos()} />
				<InfiniteLoader
					hasMore={videos.hasNextPage}
					loading={videos.isFetchingNextPage}
					failed={videos.isFetchNextPageError}
					onload={() => void videos.fetchNextPage()}
				/>
			{/if}
		</section>
	{/if}
</div>
