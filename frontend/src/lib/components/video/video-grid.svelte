<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import RowScroller from '#lib/components/row-scroller.svelte';
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
		showChannel = true,
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
		showChannel?: boolean;
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
				<VideoCardSkeleton {showChannel} />
			{/each}
		</div>
	{:else if layout === 'row'}
		<RowScroller>
			{#snippet children(scroller)}
				{@render list(scroller)}
			{/snippet}
		</RowScroller>
	{:else}
		{@render list()}
	{/if}
</div>

{#snippet list(scroller?: Attachment<HTMLElement>)}
	<ul class="video-list" data-layout={layout} aria-label={label} {@attach scroller}>
		{#each videos as video (video.id)}
			<li class="min-w-0">
				<VideoCard {video} {headingLevel} {showChannel} {now} />
			</li>
		{/each}
	</ul>
{/snippet}
