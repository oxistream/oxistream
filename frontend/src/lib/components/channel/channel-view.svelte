<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import RadioIcon from '@lucide/svelte/icons/radio';
	import SearchIcon from '@lucide/svelte/icons/search';
	import SearchXIcon from '@lucide/svelte/icons/search-x';
	import SmartphoneIcon from '@lucide/svelte/icons/smartphone';
	import UserXIcon from '@lucide/svelte/icons/user-x';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { ResolvedPathname } from '$app/types';
	import { createBlockMutation, createBlocksQuery } from '#lib/api/blocks.js';
	import {
		createChannelPlaylistsQuery,
		createChannelQuery,
		createChannelSearchQuery,
		createChannelTabQuery
	} from '#lib/api/channel.js';
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { isNotFound, uniqueBy } from '#lib/api/mapping.js';
	import type { SearchResult } from '#lib/api/search.js';
	import { isVideoTab, parseChannelSort, type ChannelTab } from '#lib/channel-tabs.js';
	import ChannelBlockedNotice from '#lib/components/block/channel-blocked-notice.svelte';
	import SearchResults from '#lib/components/search/search-results.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import ChannelHeaderSkeleton from './channel-header-skeleton.svelte';
	import ChannelHeader from './channel-header.svelte';
	import ChannelPlaylistGrid from './channel-playlist-grid.svelte';
	import ChannelSort from './channel-sort.svelte';
	import ChannelTabs from './channel-tabs.svelte';
	import ShortGrid from './short-grid.svelte';

	let {
		id,
		/** Channel path without tab, `/@handle` or `/channel/UC…`, already resolved. */
		base,
		tab = 'videos',
		/** Set while a "@handle" is still being resolved to an id. */
		resolving = false,
		/** Resolution of the handle failed. */
		resolveError,
		onretry
	}: {
		id?: string;
		base: ResolvedPathname;
		tab?: ChannelTab;
		resolving?: boolean;
		resolveError?: unknown;
		onretry?: () => void;
	} = $props();

	const channel = createChannelQuery(() => id);
	const blocks = createBlocksQuery();
	const unblock = createBlockMutation();
	const blocked = $derived(!!id && (blocks.data?.some((c) => c.id === id) ?? false));
	const ready = $derived(channel.data ? id : undefined);

	const videoTab = $derived(isVideoTab(tab) ? tab : 'videos');
	const sort = $derived(parseChannelSort(page.url.searchParams.get('sort')));
	const query = $derived(tab === 'search' ? (page.url.searchParams.get('query') ?? '').trim() : '');

	const videos = createChannelTabQuery(
		() => (isVideoTab(tab) ? ready : undefined),
		() => videoTab,
		() => sort
	);
	const playlists = createChannelPlaylistsQuery(() => (tab === 'playlists' ? ready : undefined));
	const search = createChannelSearchQuery(
		() => (tab === 'search' ? ready : undefined),
		() => query
	);

	const listed = $derived(
		uniqueBy(
			(tab === 'search' ? search : videos).data?.pages.flatMap((p) => p.items) ?? [],
			(video) => video.id
		)
	);
	const progress = createProgressLookup(() => listed.map((v) => v.id));
	const items = $derived(withProgress(listed, progress.get));
	const playlistItems = $derived(
		uniqueBy(playlists.data?.pages.flatMap((p) => p.items) ?? [], (p) => p.id)
	);
	const results = $derived<SearchResult[]>(
		items.map((video) => ({ type: 'video', key: video.id, video }))
	);

	const error = $derived(resolveError ?? (channel.data ? undefined : channel.error));
	const loading = $derived(!error && (resolving || channel.isPending));

	const empty = {
		videos: {
			icon: ListVideoIcon,
			title: m.channel_no_videos_title,
			body: m.channel_no_videos_body
		},
		shorts: {
			icon: SmartphoneIcon,
			title: m.channel_no_shorts_title,
			body: m.channel_no_shorts_body
		},
		streams: { icon: RadioIcon, title: m.channel_no_live_title, body: m.channel_no_live_body }
	};
	const tabTitle = $derived(
		{
			videos: m.channel_videos(),
			shorts: m.channel_tab_shorts(),
			streams: m.channel_tab_live(),
			playlists: m.channel_tab_playlists(),
			search: query ? m.channel_search_results({ query }) : m.channel_search_label()
		}[tab]
	);
</script>

<svelte:head>
	{#if channel.data}
		<title>
			{pageTitle(tab === 'videos' ? channel.data.name : `${tabTitle} · ${channel.data.name}`)}
		</title>
	{:else if error && isNotFound(error)}
		<title>{pageTitle(m.channel_not_found_title())}</title>
	{:else}
		<title>{pageTitle(m.channel_title())}</title>
	{/if}
</svelte:head>

<div class="flex w-full flex-col gap-6 px-(--page-gutter) py-6">
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
	{:else if blocked}
		<ChannelBlockedNotice
			channel={channel.data}
			pending={unblock.isPending}
			onunblock={() => channel.data && unblock.mutate({ channel: channel.data, block: false })}
		/>
	{:else}
		<ChannelHeader channel={channel.data} />
		<div class="flex flex-col gap-5">
			<ChannelTabs
				{base}
				current={tab}
				hasShorts={channel.data.hasShorts}
				hasLive={channel.data.hasLive}
				{query}
			/>
			<section aria-labelledby="channel-tab-heading" class="flex flex-col gap-5">
				<h2 id="channel-tab-heading" class="sr-only">{tabTitle}</h2>
				{#if isVideoTab(tab)}
					{#if items.length > 0 || sort !== 'latest'}
						<ChannelSort {base} tab={videoTab} current={sort} />
					{/if}
					{#if videos.isPending}
						{#if tab === 'shorts'}
							<ShortGrid loading />
						{:else}
							<VideoGrid loading showChannel={false} skeletonCount={8} />
						{/if}
					{:else if videos.isError && !videos.data}
						<ErrorState onretry={() => void videos.refetch()} />
					{:else if items.length === 0}
						<EmptyState
							icon={empty[videoTab].icon}
							headingLevel={3}
							title={empty[videoTab].title()}
							description={empty[videoTab].body()}
						/>
					{:else}
						{#if tab === 'shorts'}
							<ShortGrid shorts={items} label={tabTitle} />
						{:else}
							<VideoGrid videos={items} showChannel={false} label={tabTitle} />
						{/if}
						<InfiniteLoader
							hasMore={videos.hasNextPage}
							loading={videos.isFetchingNextPage}
							failed={videos.isFetchNextPageError}
							onload={() => void videos.fetchNextPage()}
						/>
					{/if}
				{:else if tab === 'playlists'}
					{#if playlists.isPending}
						<ChannelPlaylistGrid loading />
					{:else if playlists.isError && !playlists.data}
						<ErrorState onretry={() => void playlists.refetch()} />
					{:else if playlistItems.length === 0}
						<EmptyState
							icon={ListVideoIcon}
							headingLevel={3}
							title={m.channel_no_playlists_title()}
							description={m.channel_no_playlists_body()}
						/>
					{:else}
						<ChannelPlaylistGrid playlists={playlistItems} label={tabTitle} />
						<InfiniteLoader
							hasMore={playlists.hasNextPage}
							loading={playlists.isFetchingNextPage}
							failed={playlists.isFetchNextPageError}
							onload={() => void playlists.fetchNextPage()}
						/>
					{/if}
				{:else if !query}
					<EmptyState
						icon={SearchIcon}
						headingLevel={3}
						title={m.channel_search_prompt_title()}
						description={m.channel_search_prompt_body()}
					/>
				{:else if search.isPending}
					<SearchResults loading class="max-w-5xl" />
				{:else if search.isError && !search.data}
					<ErrorState
						title={m.search_error_title()}
						description={m.search_error_body()}
						onretry={() => void search.refetch()}
					/>
				{:else if results.length === 0}
					<EmptyState
						icon={SearchXIcon}
						headingLevel={3}
						title={m.search_empty_title({ query })}
						description={m.search_empty_body()}
					/>
				{:else}
					<SearchResults {results} class="max-w-5xl" />
					<InfiniteLoader
						class="max-w-5xl"
						hasMore={search.hasNextPage}
						loading={search.isFetchingNextPage}
						failed={search.isFetchNextPageError}
						onload={() => void search.fetchNextPage()}
						endLabel={m.search_end()}
					/>
				{/if}
			</section>
		</div>
	{/if}
</div>
