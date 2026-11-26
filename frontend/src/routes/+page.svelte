<script lang="ts">
	import { createTrendingStatus, type TrendingCategory } from '#lib/api/trending.js';
	import GuestFallback from '#lib/components/home/guest-fallback.svelte';
	import HomeSection from '#lib/components/home/home-section.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { recentLocalVideos } from '#lib/components/watch/local-progress.js';
	import { homeSections, showGuestFallback, trendingCategory } from '#lib/home/sections.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const RECENT_LIMIT = 12;

	const guest = $derived(session.ready && !session.user);
	const sections = $derived(homeSections(session.user ? 'user' : 'guest'));
	const categories = $derived(
		sections.map(trendingCategory).filter((c): c is TrendingCategory => c !== undefined)
	);
	const trending = createTrendingStatus(
		() => categories,
		() => guest
	);
	const fallback = $derived(guest && showGuestFallback(trending));
	const recent = $derived(fallback ? recentLocalVideos(RECENT_LIMIT) : []);
</script>

<div class="flex flex-col gap-8 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_home()} documentTitle={pageTitle()} class="sr-only" />

	{#if !session.ready}
		{#each { length: 2 }, i (i)}
			<div class="flex flex-col gap-3">
				<div class="flex min-h-8 items-center"><Skeleton class="h-5 w-40" /></div>
				<VideoGrid loading layout="row" skeletonCount={5} />
			</div>
		{/each}
	{:else if fallback}
		<GuestFallback {recent} />
	{:else}
		{#each sections as key (key)}
			<HomeSection {key} />
		{/each}
	{/if}
</div>
