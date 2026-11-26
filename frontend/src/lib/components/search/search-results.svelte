<script lang="ts">
	import type { SearchResult } from '#lib/api/search.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import ChannelResult from './channel-result.svelte';
	import PlaylistResult from './playlist-result.svelte';
	import ResultSkeleton from './result-skeleton.svelte';
	import VideoResult from './video-result.svelte';

	let {
		results = [],
		loading = false,
		skeletonCount = 6,
		now,
		class: className
	}: {
		results?: SearchResult[];
		loading?: boolean;
		skeletonCount?: number;
		now?: Date;
		class?: string;
	} = $props();
</script>

<div class={cn('@container', className)}>
	{#if loading}
		<div class="flex flex-col gap-6" role="status" aria-label={m.search_loading()}>
			{#each { length: skeletonCount }, i (i)}
				<ResultSkeleton />
			{/each}
		</div>
	{:else}
		<ul class="flex flex-col gap-6 @xl:gap-5" aria-label={m.search_results_label()}>
			{#each results as result (result.key)}
				<li class="min-w-0">
					{#if result.type === 'video'}
						<VideoResult video={result.video} {now} />
					{:else if result.type === 'channel'}
						<ChannelResult channel={result.channel} />
					{:else}
						<PlaylistResult playlist={result.playlist} />
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</div>
