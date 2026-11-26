<script lang="ts">
	import { createProgressLookup } from '#lib/api/history.js';
	import { withProgress } from '#lib/api/history-list.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';
	import RelatedVideoCard from './related-video-card.svelte';

	let {
		videos = [],
		loading = false,
		now,
		class: className
	}: {
		videos?: VideoSummary[];
		loading?: boolean;
		now?: Date;
		class?: string;
	} = $props();

	const progress = createProgressLookup(() => videos.map((v) => v.id));
	const shown = $derived(withProgress(videos, progress.get));

	const listClass = 'grid gap-3 @xl:grid-cols-2 @xl:gap-x-4 @xl:gap-y-6 @4xl:grid-cols-3';
</script>

<section aria-labelledby="related-heading" class={cn('@container flex flex-col gap-3', className)}>
	<h2 id="related-heading" class="text-base font-semibold tracking-tight">{m.watch_related()}</h2>
	{#if loading}
		<div class={listClass} role="status" aria-label={m.video_loading()}>
			{#each { length: 8 }, i (i)}
				<div class="flex gap-3 @xl:flex-col" aria-hidden="true">
					<Skeleton class="aspect-video w-40 shrink-0 rounded-md @xl:w-full @xl:rounded-lg" />
					<div class="flex flex-1 flex-col gap-2 pt-1">
						<Skeleton class="h-3.5 w-11/12" />
						<Skeleton class="h-3.5 w-2/3" />
						<Skeleton class="mt-1 h-3 w-1/2" />
					</div>
				</div>
			{/each}
		</div>
	{:else if videos.length === 0}
		<p class="text-sm text-muted-foreground">{m.watch_related_empty()}</p>
	{:else}
		<ul class={listClass}>
			{#each shown as video (video.id)}
				<li class="min-w-0"><RelatedVideoCard {video} {now} /></li>
			{/each}
		</ul>
	{/if}
</section>
