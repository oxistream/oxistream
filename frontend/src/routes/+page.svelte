<script lang="ts">
	import HouseIcon from '@lucide/svelte/icons/house';
	import { resolve } from '$app/paths';
	import { createSettingsQuery } from '#lib/api/settings.js';
	import { createSubscriptionGroupsQuery } from '#lib/api/subscription-groups.js';
	import { createTrendingStatus, type TrendingCategory } from '#lib/api/trending.js';
	import GroupSection from '#lib/components/home/group-section.svelte';
	import GuestFallback from '#lib/components/home/guest-fallback.svelte';
	import HomeSection from '#lib/components/home/home-section.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { recentLocalVideos, removeLocalProgress } from '#lib/components/watch/local-progress.js';
	import { homeSections, showGuestFallback, trendingCategory } from '#lib/home/sections.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import type { VideoSummary } from '#lib/video.js';

	const RECENT_LIMIT = 12;

	const guest = $derived(session.ready && !session.user);
	const settings = createSettingsQuery();
	const groups = createSubscriptionGroupsQuery();
	// Waits for the user's own layout, so the default does not flash first.
	const ready = $derived(session.ready && (!session.user || !settings.isPending));
	// Group rows follow the subscriptions row, or close the page without it.
	const homeGroups = $derived(
		session.user ? (groups.data ?? []).filter((g) => g.showOnHome && g.channelIds.length > 0) : []
	);
	const sections = $derived(
		session.user ? homeSections('user', settings.data?.home_sections) : homeSections('guest')
	);
	const groupsAfterSubscriptions = $derived(sections.includes('subscriptions'));
	const categories = $derived(
		sections.map(trendingCategory).filter((c): c is TrendingCategory => c !== undefined)
	);
	const trending = createTrendingStatus(
		() => categories,
		() => guest
	);
	const fallback = $derived(guest && showGuestFallback(trending));
	// Bumped on removal, since local storage is not reactive.
	let localVersion = $state(0);
	const recent = $derived.by(() => {
		void localVersion;
		return fallback ? recentLocalVideos(RECENT_LIMIT) : [];
	});

	function removeRecent(video: VideoSummary) {
		removeLocalProgress(video.id);
		localVersion++;
	}
</script>

<div class="flex flex-col gap-8 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_home()} documentTitle={pageTitle()} class="sr-only" />

	{#if !ready}
		{#each { length: 2 }, i (i)}
			<div class="flex flex-col gap-3">
				<div class="flex min-h-8 items-center"><Skeleton class="h-5 w-40" /></div>
				<VideoGrid loading layout="row" skeletonCount={5} />
			</div>
		{/each}
	{:else if fallback}
		<GuestFallback {recent} onremove={removeRecent} />
	{:else if sections.length === 0 && homeGroups.length === 0}
		<EmptyState icon={HouseIcon} title={m.home_empty_title()} description={m.home_empty_body()}>
			{#snippet actions()}
				<Button href={resolve('/settings')} variant="outline">{m.home_empty_action()}</Button>
			{/snippet}
		</EmptyState>
	{:else}
		{#each sections as key (key)}
			<HomeSection {key} />
			{#if key === 'subscriptions'}{@render groupRows()}{/if}
		{/each}
		{#if !groupsAfterSubscriptions}{@render groupRows()}{/if}
	{/if}
</div>

{#snippet groupRows()}
	{#each homeGroups as group (group.id)}
		<GroupSection {group} />
	{/each}
{/snippet}
