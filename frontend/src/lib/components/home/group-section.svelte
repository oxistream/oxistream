<script lang="ts">
	import { resolve } from '$app/paths';
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import type { SubscriptionGroup } from '#lib/api/subscription-group-list.js';
	import { createFeedQuery } from '#lib/api/subscriptions.js';
	import { rowState } from '#lib/home/sections.js';
	import HomeRow from './home-row.svelte';

	const LIMIT = 15;

	let { group }: { group: SubscriptionGroup } = $props();

	// Shares its cache with the group's tab on the subscriptions feed page.
	const feed = createFeedQuery(() => false, 'videos', { filters: () => ({ group: group.id }) });
	const listed = $derived(
		uniqueBy(feed.data?.pages.flatMap((p) => p.items) ?? [], (video) => video.id).slice(0, LIMIT)
	);
	const progress = createProgressLookup(() => listed.map((v) => v.id));
	const videos = $derived(withProgress(listed, progress.get));
	const state = $derived(rowState({ isPending: feed.isPending, count: videos.length }));
</script>

{#if state !== 'hidden'}
	<HomeRow
		title={group.name}
		href={resolve(`/feed/subscriptions?group=${group.id}`)}
		{videos}
		loading={state === 'loading'}
	/>
{/if}
