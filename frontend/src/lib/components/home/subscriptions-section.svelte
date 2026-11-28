<script lang="ts">
	import HourglassIcon from '@lucide/svelte/icons/hourglass';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { resolve } from '$app/paths';
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { hasPendingChannels } from '#lib/api/subscription-list.js';
	import { createFeedQuery, createSubscriptionsQuery } from '#lib/api/subscriptions.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import HomeRow from './home-row.svelte';
	import NoSubscriptions from './no-subscriptions.svelte';

	const LIMIT = 15;

	const subscriptions = createSubscriptionsQuery();
	const pending = $derived(subscriptions.data ? hasPendingChannels(subscriptions.data) : false);
	// Shares its cache with the subscriptions feed page.
	const feed = createFeedQuery(() => pending);

	const listed = $derived(
		uniqueBy(feed.data?.pages.flatMap((p) => p.items) ?? [], (video) => video.id).slice(0, LIMIT)
	);
	const progress = createProgressLookup(() => listed.map((v) => v.id));
	const videos = $derived(withProgress(listed, progress.get));
	const title = $derived(m.home_latest_subscriptions());
</script>

{#if subscriptions.data?.length === 0}
	<HomeRow {title}>
		<NoSubscriptions />
	</HomeRow>
{:else if feed.isError && !feed.data}
	<HomeRow {title}>
		<ErrorState onretry={() => void feed.refetch()} class="py-10" />
	</HomeRow>
{:else if !feed.isPending && !subscriptions.isPending && videos.length === 0}
	<HomeRow {title}>
		<EmptyState
			icon={pending ? HourglassIcon : TvMinimalPlayIcon}
			title={pending ? m.feed_fetching_title() : m.feed_no_videos_title()}
			description={pending ? m.feed_fetching_body() : undefined}
			headingLevel={3}
		/>
	</HomeRow>
{:else}
	<HomeRow
		{title}
		href={resolve('/feed/subscriptions')}
		{videos}
		loading={feed.isPending || subscriptions.isPending}
	/>
{/if}
