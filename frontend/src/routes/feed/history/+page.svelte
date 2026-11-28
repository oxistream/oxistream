<script lang="ts">
	import ClockIcon from '@lucide/svelte/icons/clock';
	import PauseIcon from '@lucide/svelte/icons/pause';
	import PlayIcon from '@lucide/svelte/icons/play';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import {
		createClearHistoryMutation,
		createHistoryPausedMutation,
		createHistoryQuery,
		createHistorySettingsQuery,
		createRemoveFromHistoryMutation
	} from '#lib/api/history.js';
	import { groupByDay } from '#lib/api/history-list.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import ClearHistoryDialog from '#lib/components/history/clear-history-dialog.svelte';
	import HistoryList from '#lib/components/history/history-list.svelte';
	import HistoryPausedNotice from '#lib/components/history/history-paused-notice.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	const SEARCH_DEBOUNCE_MS = 300;

	const query = $derived((page.url.searchParams.get('q') ?? '').trim());
	let input = $state(page.url.searchParams.get('q') ?? '');
	// The last term written to the URL, so typing on is not reset by our own navigation.
	let pushed = (page.url.searchParams.get('q') ?? '').trim();

	$effect(() => {
		const value = input.trim();
		if (value === pushed) return;
		const timer = setTimeout(() => {
			pushed = value;
			const search = value ? `?${new URLSearchParams({ q: value })}` : '';
			// eslint-disable-next-line svelte/no-navigation-without-resolve -- path resolved inline
			void goto(`${resolve('/feed/history')}${search}`, { replace: true, reset: false });
		}, SEARCH_DEBOUNCE_MS);
		return () => clearTimeout(timer);
	});

	// Links to the page change the term without typing.
	$effect(() => {
		if (query !== pushed) {
			pushed = query;
			input = query;
		}
	});

	const history = createHistoryQuery(() => query);
	const settings = createHistorySettingsQuery();
	const remove = createRemoveFromHistoryMutation();
	const clear = createClearHistoryMutation();
	const pause = createHistoryPausedMutation();

	const items = $derived(
		uniqueBy(history.data?.pages.flatMap((p) => p.items) ?? [], (video) => video.id)
	);
	const days = $derived(
		groupByDay(items, {
			locale: getLocale(),
			today: m.history_today(),
			yesterday: m.history_yesterday()
		})
	);
	const paused = $derived(settings.data?.paused === true);
</script>

<div class="page-column flex flex-col gap-6 py-6">
	<PageHeader title={m.nav_history()}>
		{#snippet actions()}
			{#if session.user}
				<Button
					variant="outline"
					size="sm"
					disabled={settings.isPending}
					aria-pressed={paused}
					onclick={() => pause.mutate(!paused)}
				>
					{#if paused}<PlayIcon />{m.history_resume()}{:else}<PauseIcon />{m.history_pause()}{/if}
				</Button>
				<ClearHistoryDialog
					disabled={items.length === 0 && !query}
					onconfirm={() => clear.mutate()}
				/>
			{/if}
		{/snippet}
	</PageHeader>

	{#if session.user && paused}<HistoryPausedNotice />{/if}

	{#if session.user}
		<div class="relative w-full sm:max-w-sm">
			<SearchIcon
				class="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
				aria-hidden="true"
			/>
			<Input
				type="search"
				bind:value={input}
				placeholder={m.history_search()}
				aria-label={m.history_search()}
				class="ps-9"
				autocomplete="off"
			/>
		</div>
	{/if}

	{#if !session.ready}
		<HistoryList loading />
	{:else if !session.user}
		<EmptyState
			icon={ClockIcon}
			title={m.empty_history_title()}
			description={m.empty_sign_in_hint()}
		>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{:else if history.isPending}
		<HistoryList loading />
	{:else if history.isError && !history.data}
		<ErrorState onretry={() => void history.refetch()} />
	{:else if items.length === 0 && query}
		<EmptyState icon={SearchIcon} title={m.search_empty_title({ query })} />
	{:else if items.length === 0}
		<EmptyState
			icon={ClockIcon}
			title={m.empty_history_title()}
			description={m.empty_history_body()}
		/>
	{:else}
		<HistoryList {days} onremove={(id) => remove.mutate(id)} />
		<InfiniteLoader
			hasMore={history.hasNextPage}
			loading={history.isFetchingNextPage}
			failed={history.isFetchNextPageError}
			onload={() => void history.fetchNextPage()}
			endLabel={m.history_end()}
		/>
	{/if}
</div>
