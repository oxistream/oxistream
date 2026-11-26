<script lang="ts">
	import { formatDuration } from '#lib/format.js';
	import { cn } from '#lib/utils.js';

	let {
		src,
		durationSeconds,
		progress,
		badge,
		class: className
	}: {
		src?: string;
		durationSeconds?: number;
		progress?: number;
		/** Replaces the duration chip, e.g. for live streams. */
		badge?: string;
		class?: string;
	} = $props();

	const percent = $derived(
		progress === undefined ? undefined : Math.round(Math.min(1, Math.max(0, progress)) * 100)
	);
</script>

<!-- Purely visual: the card's link text carries the title, duration and progress for assistive tech. -->
<div
	class={cn('relative aspect-video overflow-hidden rounded-lg bg-muted', className)}
	aria-hidden="true"
>
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
	{/if}
	{#if badge}
		<span
			class="absolute end-1.5 bottom-1.5 rounded-sm bg-primary px-1.5 py-px text-xs font-semibold text-primary-foreground uppercase"
		>
			{badge}
		</span>
	{:else if durationSeconds !== undefined}
		<span
			class={cn(
				'absolute end-1.5 rounded-sm bg-overlay px-1.5 py-px text-xs font-medium text-overlay-foreground tabular',
				percent === undefined ? 'bottom-1.5' : 'bottom-2.5'
			)}
		>
			{formatDuration(durationSeconds)}
		</span>
	{/if}
	{#if percent !== undefined}
		<div class="absolute inset-x-0 bottom-0 h-1 bg-white/35">
			<div class="h-full bg-primary" style:width="{percent}%"></div>
		</div>
	{/if}
</div>
