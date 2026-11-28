<script lang="ts">
	import ArrowDownUpIcon from '@lucide/svelte/icons/arrow-down-up';
	import FunnelXIcon from '@lucide/svelte/icons/funnel-x';
	import HourglassIcon from '@lucide/svelte/icons/hourglass';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import UsersIcon from '@lucide/svelte/icons/users';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { createSettingsMutation, createSettingsQuery } from '#lib/api/settings.js';
	import { hasPendingChannels } from '#lib/api/subscription-list.js';
	import {
		createFeedQuery,
		createSubscriptionsQuery,
		subscriptionKeys
	} from '#lib/api/subscriptions.js';
	import ShortRow from '#lib/components/shorts/short-row.svelte';
	import FeedFilters from '#lib/components/subscription/feed-filters.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const client = useQueryClient();
	const subscriptions = createSubscriptionsQuery();
	const pending = $derived(subscriptions.data ? hasPendingChannels(subscriptions.data) : false);

	// Filter toggles follow the account; the channel lives in the URL, so it can be linked.
	const settings = createSettingsQuery();
	const saveSettings = createSettingsMutation();
	const settingsReady = $derived(settings.isSuccess || settings.isError);
	const hideWatched = $derived(settings.data?.feed_hide_watched ?? false);
	const hideShorts = $derived(settings.data?.feed_hide_shorts ?? false);
	const channel = $derived(
		page.url.searchParams.get('channel')?.match(/^UC[\w-]{22}$/)?.[0] ?? undefined
	);
	const filtered = $derived(hideWatched || hideShorts || channel !== undefined);

	const feed = createFeedQuery(() => pending, 'videos', {
		filters: () => ({ hideWatched, channel }),
		enabled: () => settingsReady
	});
	const shortsFeed = createFeedQuery(() => pending, 'shorts', {
		filters: () => ({ channel }),
		enabled: () => settingsReady && !hideShorts
	});
	const shorts = $derived(
		hideShorts
			? []
			: uniqueBy(shortsFeed.data?.pages.flatMap((p) => p.items) ?? [], (short) => short.id)
	);

	const listed = $derived(
		uniqueBy(feed.data?.pages.flatMap((p) => p.items) ?? [], (video) => video.id)
	);
	const progress = createProgressLookup(() => listed.map((v) => v.id));
	const items = $derived(withProgress(listed, progress.get));
	const noSubscriptions = $derived(subscriptions.data?.length === 0);

	function setChannel(id: string | undefined) {
		const target = id
			? resolve(`/feed/subscriptions?channel=${id}`)
			: resolve('/feed/subscriptions');
		void goto(target, { reset: false });
	}

	function clearFilters() {
		saveSettings.mutate({ feed_hide_watched: false, feed_hide_shorts: false });
		setChannel(undefined);
	}

	// The last pending channel just landed: load the feed once more to pick up its videos.
	let wasPending = false;
	$effect(() => {
		if (wasPending && !pending) void client.invalidateQueries({ queryKey: subscriptionKeys.feed });
		wasPending = pending;
	});
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_subscriptions()}>
		{#snippet actions()}
			{#if session.user && subscriptions.data?.length}
				<Button href={resolve('/feed/channels')} variant="outline" size="sm">
					<UsersIcon />
					{m.feed_manage()}
				</Button>
			{/if}
		{/snippet}
	</PageHeader>

	{#if !session.ready}
		<VideoGrid loading skeletonCount={12} />
	{:else if !session.user}
		<EmptyState
			icon={TvMinimalPlayIcon}
			title={m.empty_subscriptions_title()}
			description={m.empty_sign_in_hint()}
		>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{:else if noSubscriptions}
		<EmptyState
			icon={TvMinimalPlayIcon}
			title={m.empty_subscriptions_title()}
			description={m.empty_subscriptions_body()}
		>
			{#snippet actions()}
				<Button href={resolve('/subscriptions/import-export')} variant="outline">
					<ArrowDownUpIcon />
					{m.subscriptions_import_export()}
				</Button>
			{/snippet}
		</EmptyState>
	{:else}
		{#if subscriptions.data?.length}
			<FeedFilters
				{hideWatched}
				{hideShorts}
				{channel}
				channels={subscriptions.data}
				onhidewatched={(hide) => saveSettings.mutate({ feed_hide_watched: hide })}
				onhideshorts={(hide) => saveSettings.mutate({ feed_hide_shorts: hide })}
				onchannel={setChannel}
			/>
		{/if}
		{@render content()}
	{/if}
</div>

{#snippet content()}
	{#if feed.isPending || subscriptions.isPending}
		<VideoGrid loading skeletonCount={12} />
	{:else if feed.isError && !feed.data}
		<ErrorState onretry={() => void feed.refetch()} />
	{:else if items.length === 0 && shorts.length === 0 && filtered && !pending}
		<EmptyState icon={FunnelXIcon} title={m.feed_filtered_empty_title()}>
			{#snippet actions()}
				<Button variant="outline" onclick={clearFilters} data-testid="feed-filters-reset">
					{m.search_filters_clear()}
				</Button>
			{/snippet}
		</EmptyState>
	{:else if items.length === 0 && shorts.length === 0}
		<EmptyState
			icon={pending ? HourglassIcon : TvMinimalPlayIcon}
			title={pending ? m.feed_fetching_title() : m.feed_no_videos_title()}
			description={pending ? m.feed_fetching_body() : m.feed_no_videos_body()}
		/>
	{:else}
		{#if pending}
			<p class="flex items-center gap-2 text-sm text-muted-foreground" role="status">
				<LoaderCircleIcon
					class="size-4 animate-spin motion-reduce:animate-none"
					aria-hidden="true"
				/>
				{m.feed_fetching_inline()}
			</p>
		{/if}
		{#if shorts.length}
			<section aria-labelledby="feed-shorts" class="flex flex-col gap-3">
				<h2 id="feed-shorts" class="text-base font-semibold tracking-tight">{m.feed_shorts()}</h2>
				<ShortRow {shorts} label={m.feed_shorts()} />
			</section>
		{/if}
		<VideoGrid videos={items} label={m.nav_subscriptions()} hideable />
		<InfiniteLoader
			hasMore={feed.hasNextPage}
			loading={feed.isFetchingNextPage}
			failed={feed.isFetchNextPageError}
			onload={() => void feed.fetchNextPage()}
			endLabel={m.feed_end()}
		/>
	{/if}
{/snippet}
