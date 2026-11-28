<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import type { Snippet } from 'svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { VideoRemoval, VideoSummary } from '#lib/video.js';

	let {
		title,
		href,
		videos = [],
		loading = false,
		now,
		removal,
		children
	}: {
		title: string;
		/** Target of the "Show all" link; no link without it. */
		href?: string;
		videos?: VideoSummary[];
		loading?: boolean;
		now?: Date;
		removal?: VideoRemoval;
		/** Replaces the row, e.g. with an empty or error state. */
		children?: Snippet;
	} = $props();

	const headingId = $props.id();
</script>

<section aria-labelledby={headingId} class="flex flex-col gap-3">
	<div class="flex min-h-8 items-center justify-between gap-4">
		<h2 id={headingId} class="text-base font-semibold tracking-tight">{title}</h2>
		{#if href && !children}
			<Button {href} variant="ghost" size="sm" class="-me-2">
				{m.home_show_all()}
				<ArrowRightIcon />
			</Button>
		{/if}
	</div>
	{#if children}
		{@render children()}
	{:else}
		<!-- Every home row leaves hidden videos out. -->
		<VideoGrid
			{videos}
			{loading}
			{now}
			skeletonCount={5}
			layout="row"
			label={title}
			{removal}
			hideable
		/>
	{/if}
</section>
