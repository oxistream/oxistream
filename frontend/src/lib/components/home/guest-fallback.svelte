<script lang="ts">
	import SearchBox from '#lib/components/search/search-box.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { VideoSummary } from '#lib/video.js';
	import HomeRow from './home-row.svelte';

	let {
		recent = [],
		now,
		onremove
	}: {
		/** Videos watched in this browser, most recent first. */
		recent?: VideoSummary[];
		now?: Date;
		onremove?: (video: VideoSummary) => void;
	} = $props();

	const headingId = $props.id();
</script>

<section
	aria-labelledby={headingId}
	class="flex flex-col items-center gap-5 pt-10 pb-4 text-center sm:pt-16 sm:pb-8"
>
	<div class="flex max-w-lg flex-col gap-2">
		<h2 id={headingId} class="text-2xl font-semibold tracking-tight text-balance">
			{m.home_search_title()}
		</h2>
		<p class="text-sm text-pretty text-muted-foreground">{m.home_search_body()}</p>
	</div>
	<SearchBox size="lg" class="w-full max-w-2xl text-start" />
</section>

{#if recent.length}
	<HomeRow
		title={m.home_recently_watched()}
		videos={recent}
		{now}
		removal={onremove && { label: m.action_remove(), onremove }}
	/>
{/if}
