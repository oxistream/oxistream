<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';
	import VideoCardSkeleton from './video-card-skeleton.svelte';
	import VideoCard from './video-card.svelte';

	let {
		videos = [],
		loading = false,
		skeletonCount = 8,
		layout = 'grid',
		headingLevel = 3,
		now,
		label,
		class: className
	}: {
		videos?: VideoSummary[];
		loading?: boolean;
		skeletonCount?: number;
		/** `row` is a single line that scrolls sideways, for shelves such as "Continue watching". */
		layout?: 'grid' | 'row';
		headingLevel?: 2 | 3 | 4;
		/** Reference time for relative dates; fixed in stories. */
		now?: Date;
		label?: string;
		class?: string;
	} = $props();
</script>

<div class={cn('@container', className)}>
	{#if loading}
		<div class="video-list" data-layout={layout} role="status" aria-label={m.video_loading()}>
			{#each { length: skeletonCount }, i (i)}
				<VideoCardSkeleton />
			{/each}
		</div>
	{:else}
		<ul class="video-list" data-layout={layout} aria-label={label}>
			{#each videos as video (video.id)}
				<li class="min-w-0">
					<VideoCard {video} {headingLevel} {now} />
				</li>
			{/each}
		</ul>
	{/if}
</div>
