<script lang="ts">
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';
	import { rememberShortsList } from '#lib/shorts/queue.js';
	import ShortCard from './short-card.svelte';

	let {
		shorts = [],
		loading = false,
		skeletonCount = 12,
		label,
		playAsList = false,
		class: className
	}: {
		shorts?: VideoSummary[];
		loading?: boolean;
		skeletonCount?: number;
		label?: string;
		/** Opening a short plays through this grid instead of the short's channel. */
		playAsList?: boolean;
		class?: string;
	} = $props();

	const grid =
		'grid grid-cols-2 gap-x-3 gap-y-5 @md:grid-cols-3 @2xl:grid-cols-4 @4xl:grid-cols-5 @6xl:grid-cols-6 @[96rem]:grid-cols-7';
</script>

<div class={cn('@container', className)}>
	{#if loading}
		<div class={grid} role="status" aria-label={m.video_loading()}>
			{#each { length: skeletonCount }, i (i)}
				<div class="flex flex-col gap-2">
					<Skeleton class="aspect-9/16 w-full rounded-lg" />
					<Skeleton class="h-4 w-4/5" />
					<Skeleton class="h-3 w-1/3" />
				</div>
			{/each}
		</div>
	{:else}
		<ul class={grid} aria-label={label}>
			{#each shorts as short (short.id)}
				<li class="min-w-0">
					<ShortCard {short} onopen={playAsList ? () => rememberShortsList(shorts) : undefined} />
				</li>
			{/each}
		</ul>
	{/if}
</div>
