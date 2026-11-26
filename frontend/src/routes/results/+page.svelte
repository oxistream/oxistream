<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import SearchXIcon from '@lucide/svelte/icons/search-x';
	import { page } from '$app/state';
	import { createProgressLookup } from '#lib/api/history.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { createSearchQuery } from '#lib/api/search.js';
	import ChannelPlaylistGrid from '#lib/components/channel/channel-playlist-grid.svelte';
	import ShortGrid from '#lib/components/channel/short-grid.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import SearchResults from '#lib/components/search/search-results.svelte';
	import SearchTabs from '#lib/components/search/search-tabs.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { parseSearchType, type SearchType } from '#lib/search-types.js';

	const query = $derived((page.url.searchParams.get('search_query') ?? '').trim());
	const type = $derived(parseSearchType(page.url.searchParams.get('type')));
	const search = createSearchQuery(
		() => query,
		() => type
	);

	const found = $derived(
		uniqueBy(search.data?.pages.flatMap((p) => p.items) ?? [], (result) => result.key)
	);
	const progress = createProgressLookup(() =>
		found.flatMap((r) => (r.type === 'video' ? [r.video.id] : []))
	);
	const unique = $derived(
		found.map((r) => {
			const watched = r.type === 'video' ? progress.get(r.video.id) : undefined;
			return watched === undefined || r.type !== 'video'
				? r
				: { ...r, video: { ...r.video, progress: watched } };
		})
	);
	const shorts = $derived(unique.flatMap((r) => (r.type === 'video' ? [r.video] : [])));
	const playlists = $derived(unique.flatMap((r) => (r.type === 'playlist' ? [r.playlist] : [])));
	const corrected = $derived(search.data?.pages[0]?.correctedQuery);
	const title = $derived(query ? m.search_results_title({ query }) : m.search_placeholder());

	const emptyTitles: Record<SearchType, (inputs: { query: string }) => string> = {
		all: m.search_empty_title,
		videos: m.search_empty_videos,
		shorts: m.search_empty_shorts,
		channels: m.search_empty_channels,
		playlists: m.search_empty_playlists
	};
</script>

<div class="flex w-full max-w-5xl flex-col gap-5 px-(--page-gutter) py-6">
	<PageHeader {title} documentTitle={pageTitle(query || m.search_placeholder())} class="sr-only" />

	{#if !query}
		<EmptyState
			icon={SearchIcon}
			title={m.search_prompt_title()}
			description={m.search_prompt_body()}
		/>
	{:else}
		<SearchTabs {query} current={type} />
		{#if search.isPending}
			{#if type === 'shorts'}
				<ShortGrid loading />
			{:else if type === 'playlists'}
				<ChannelPlaylistGrid loading />
			{:else}
				<SearchResults loading />
			{/if}
		{:else if search.isError && !search.data}
			<ErrorState
				title={m.search_error_title()}
				description={m.search_error_body()}
				onretry={() => void search.refetch()}
			/>
		{:else if unique.length === 0 && !search.hasNextPage}
			<EmptyState
				icon={SearchXIcon}
				title={emptyTitles[type]({ query })}
				description={m.search_empty_body()}
			/>
		{:else}
			{#if corrected}
				<p class="text-sm text-muted-foreground" data-testid="corrected-query">
					{m.search_corrected({ query: corrected })}
				</p>
			{/if}
			{#if type === 'shorts'}
				<ShortGrid {shorts} label={m.search_results_label()} playAsList />
			{:else if type === 'playlists'}
				<ChannelPlaylistGrid {playlists} label={m.search_results_label()} />
			{:else}
				<SearchResults results={unique} />
			{/if}
			<InfiniteLoader
				hasMore={search.hasNextPage}
				loading={search.isFetchingNextPage}
				failed={search.isFetchNextPageError}
				onload={() => void search.fetchNextPage()}
				endLabel={m.search_end()}
			/>
		{/if}
	{/if}
</div>
