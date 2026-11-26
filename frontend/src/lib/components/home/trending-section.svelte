<script lang="ts">
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { createTrendingQuery, type TrendingCategory } from '#lib/api/trending.js';
	import { rowState } from '#lib/home/sections.js';
	import { m } from '#lib/paraglide/messages.js';
	import HomeRow from './home-row.svelte';

	let { category }: { category: TrendingCategory } = $props();

	const titles: Record<TrendingCategory, () => string> = {
		news: m.home_trending_news,
		gaming: m.home_trending_gaming,
		podcasts: m.home_trending_podcasts
	};

	// Categories can list hundreds of videos; a row only needs a screenful or two.
	const LIMIT = 24;

	const query = createTrendingQuery(() => category);
	const listed = $derived(query.data?.slice(0, LIMIT) ?? []);
	const progress = createProgressLookup(() => listed.map((video) => video.id));
	const videos = $derived(withProgress(listed, progress.get));
	const state = $derived(rowState({ isPending: query.isPending, count: videos.length }));
</script>

{#if state !== 'hidden'}
	<HomeRow title={titles[category]()} {videos} loading={state === 'loading'} />
{/if}
