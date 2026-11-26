<script lang="ts">
	import ArrowDownUpIcon from '@lucide/svelte/icons/arrow-down-up';
	import HourglassIcon from '@lucide/svelte/icons/hourglass';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import UsersIcon from '@lucide/svelte/icons/users';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { resolve } from '$app/paths';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { hasPendingChannels } from '#lib/api/subscription-list.js';
	import {
		createFeedQuery,
		createSubscriptionsQuery,
		subscriptionKeys
	} from '#lib/api/subscriptions.js';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { importExportHref } from '#lib/components/subscription/links.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const client = useQueryClient();
	const subscriptions = createSubscriptionsQuery();
	const pending = $derived(subscriptions.data ? hasPendingChannels(subscriptions.data) : false);
	const feed = createFeedQuery(() => pending);

	const items = $derived(
		uniqueBy(feed.data?.pages.flatMap((p) => p.items) ?? [], (video) => video.id)
	);
	const noSubscriptions = $derived(subscriptions.data?.length === 0);

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
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in navigation.ts -->
				<Button href={importExportHref()} variant="outline">
					<ArrowDownUpIcon />
					{m.subscriptions_import_export()}
				</Button>
			{/snippet}
		</EmptyState>
	{:else if feed.isPending || subscriptions.isPending}
		<VideoGrid loading skeletonCount={12} />
	{:else if feed.isError && !feed.data}
		<ErrorState onretry={() => void feed.refetch()} />
	{:else if items.length === 0}
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
		<VideoGrid videos={items} label={m.nav_subscriptions()} />
		<InfiniteLoader
			hasMore={feed.hasNextPage}
			loading={feed.isFetchingNextPage}
			failed={feed.isFetchNextPageError}
			onload={() => void feed.fetchNextPage()}
			endLabel={m.feed_end()}
		/>
	{/if}
</div>
