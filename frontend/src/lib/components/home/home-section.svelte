<script lang="ts">
	import { trendingCategory, type HomeSectionKey } from '#lib/home/sections.js';
	import ContinueWatchingSection from './continue-watching-section.svelte';
	import SubscriptionShortsSection from './subscription-shorts-section.svelte';
	import SubscriptionsSection from './subscriptions-section.svelte';
	import TrendingSection from './trending-section.svelte';

	let { key }: { key: HomeSectionKey } = $props();

	const category = $derived(trendingCategory(key));
</script>

{#if key === 'continue-watching'}
	<ContinueWatchingSection />
{:else if key === 'subscriptions'}
	<SubscriptionsSection />
{:else if key === 'subscription-shorts'}
	<SubscriptionShortsSection />
{:else if category}
	<TrendingSection {category} />
{/if}
