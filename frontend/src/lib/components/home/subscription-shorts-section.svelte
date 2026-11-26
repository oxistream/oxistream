<script lang="ts">
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { createFeedQuery } from '#lib/api/subscriptions.js';
	import ShortRow from '#lib/components/shorts/short-row.svelte';
	import { rowState } from '#lib/home/sections.js';
	import { m } from '#lib/paraglide/messages.js';
	import HomeRow from './home-row.svelte';

	const LIMIT = 20;

	// Shares its cache with the shelf on the subscriptions feed page.
	const feed = createFeedQuery(() => false, 'shorts');
	const listed = $derived(
		uniqueBy(feed.data?.pages.flatMap((p) => p.items) ?? [], (short) => short.id).slice(0, LIMIT)
	);
	const progress = createProgressLookup(() => listed.map((s) => s.id));
	const shorts = $derived(withProgress(listed, progress.get));
	const state = $derived(rowState({ isPending: feed.isPending, count: shorts.length }));
	const title = $derived(m.home_subscription_shorts());
</script>

{#if state !== 'hidden'}
	<HomeRow {title}>
		<ShortRow {shorts} loading={state === 'loading'} label={title} skeletonCount={9} />
	</HomeRow>
{/if}
