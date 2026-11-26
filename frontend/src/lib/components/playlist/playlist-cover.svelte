<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import type { Component } from 'svelte';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		src,
		count,
		icon: Icon = ListVideoIcon,
		class: className
	}: {
		src?: string;
		/** Left out when YouTube does not say how many videos there are. */
		count?: number;
		/** Shown without a thumbnail. */
		icon?: Component<{ class?: string }>;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<!-- Purely visual: a stacked thumbnail with the item count. -->
<div class={cn('flex flex-col', className)} aria-hidden="true">
	<div class="mx-3 h-1.5 rounded-t-md bg-muted-foreground/20"></div>
	<div class="relative aspect-video overflow-hidden rounded-lg bg-muted">
		{#if src}
			<img
				{src}
				alt=""
				loading="lazy"
				decoding="async"
				width="640"
				height="360"
				class="size-full object-cover transition-transform duration-(--motion-image) ease-ui group-hover/video:scale-[1.03]"
			/>
		{:else}
			<div class="flex size-full items-center justify-center text-muted-foreground">
				<Icon class="size-8" />
			</div>
		{/if}
		{#if count !== undefined}
			<span
				class="absolute end-1.5 bottom-1.5 inline-flex items-center gap-1 rounded-sm bg-overlay px-1.5 py-px text-xs font-medium text-overlay-foreground tabular"
			>
				<ListVideoIcon class="size-3.5" />
				{m.playlist_video_count({ count, formatted: formatNumber(count, locale) })}
			</span>
		{/if}
	</div>
</div>
