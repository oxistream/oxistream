<script lang="ts">
	import ShortCard from '#lib/components/channel/short-card.svelte';
	import RowScroller from '#lib/components/row-scroller.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { rememberShortsList } from '#lib/shorts/queue.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';

	let {
		shorts = [],
		loading = false,
		skeletonCount = 8,
		label,
		class: className
	}: {
		shorts?: VideoSummary[];
		loading?: boolean;
		skeletonCount?: number;
		label?: string;
		class?: string;
	} = $props();
</script>

<!-- A single line of portrait cards that scrolls sideways; opening one plays the row in order. -->
<div class={cn('@container', className)}>
	{#if loading}
		<div class="short-row" role="status" aria-label={m.video_loading()}>
			{#each { length: skeletonCount }, i (i)}
				<div class="flex flex-col gap-2">
					<Skeleton class="aspect-9/16 w-full rounded-lg" />
					<Skeleton class="h-4 w-4/5" />
					<Skeleton class="h-3 w-1/3" />
				</div>
			{/each}
		</div>
	{:else}
		<RowScroller>
			{#snippet children(scroller)}
				<ul class="short-row" aria-label={label} {@attach scroller}>
					{#each shorts as short (short.id)}
						<li class="min-w-0">
							<ShortCard {short} onopen={() => rememberShortsList(shorts)} />
						</li>
					{/each}
				</ul>
			{/snippet}
		</RowScroller>
	{/if}
</div>

<style>
	.short-row {
		--cols: 2.4;
		--gap: 0.75rem;
		display: flex;
		gap: var(--gap);
		overflow-x: auto;
		scroll-snap-type: x mandatory;
		scroll-padding-inline: var(--page-gutter);
		scrollbar-width: none;
		margin-inline: calc(var(--page-gutter) * -1);
		margin-block-start: -0.375rem;
		padding: 0.375rem var(--page-gutter) 0.75rem;
	}

	.short-row::-webkit-scrollbar {
		display: none;
	}

	.short-row > :global(*) {
		flex: 0 0 calc((100% - (var(--cols) - 1) * var(--gap)) / var(--cols));
		scroll-snap-align: start;
	}

	@container (min-width: 34rem) {
		.short-row {
			--cols: 4;
		}
	}

	@container (min-width: 52rem) {
		.short-row {
			--cols: 6;
		}
	}

	@container (min-width: 70rem) {
		.short-row {
			--cols: 7;
		}
	}

	@container (min-width: 92rem) {
		.short-row {
			--cols: 9;
		}
	}
</style>
