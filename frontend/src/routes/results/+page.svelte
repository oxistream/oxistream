<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import SearchXIcon from '@lucide/svelte/icons/search-x';
	import { page } from '$app/state';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { createSearchQuery } from '#lib/api/search.js';
	import PageHeader from '#lib/components/page-header.svelte';
	import SearchResults from '#lib/components/search/search-results.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';

	const query = $derived((page.url.searchParams.get('search_query') ?? '').trim());
	const search = createSearchQuery(() => query);

	const unique = $derived(
		uniqueBy(search.data?.pages.flatMap((p) => p.items) ?? [], (result) => result.key)
	);
	const corrected = $derived(search.data?.pages[0]?.correctedQuery);
	const title = $derived(query ? m.search_results_title({ query }) : m.search_placeholder());
</script>

<div class="flex w-full max-w-5xl flex-col gap-5 px-(--page-gutter) py-6">
	<PageHeader {title} documentTitle={pageTitle(query || m.search_placeholder())} class="sr-only" />

	{#if !query}
		<EmptyState
			icon={SearchIcon}
			title={m.search_prompt_title()}
			description={m.search_prompt_body()}
		/>
	{:else if search.isPending}
		<SearchResults loading />
	{:else if search.isError && !search.data}
		<ErrorState
			title={m.search_error_title()}
			description={m.search_error_body()}
			onretry={() => void search.refetch()}
		/>
	{:else if unique.length === 0}
		<EmptyState
			icon={SearchXIcon}
			title={m.search_empty_title({ query })}
			description={m.search_empty_body()}
		/>
	{:else}
		{#if corrected}
			<p class="text-sm text-muted-foreground" data-testid="corrected-query">
				{m.search_corrected({ query: corrected })}
			</p>
		{/if}
		<SearchResults results={unique} />
		<InfiniteLoader
			hasMore={search.hasNextPage}
			loading={search.isFetchingNextPage}
			failed={search.isFetchNextPageError}
			onload={() => void search.fetchNextPage()}
			endLabel={m.search_end()}
		/>
	{/if}
</div>
