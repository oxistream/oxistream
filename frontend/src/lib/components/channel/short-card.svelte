<script lang="ts">
	import { resolve } from '$app/paths';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';

	let {
		short,
		headingLevel = 3,
		onopen,
		class: className
	}: {
		short: VideoSummary;
		headingLevel?: 2 | 3 | 4;
		/** Called when the short is opened, e.g. to queue up the list it is in. */
		onopen?: () => void;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<article
	class={cn(
		'group/video relative flex min-w-0 flex-col gap-2 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid',
		className
	)}
>
	<div
		class="relative aspect-9/16 overflow-hidden rounded-lg bg-muted"
		aria-hidden="true"
		data-thumbnail
	>
		{#if short.thumbnailUrl}
			<img
				src={short.thumbnailUrl}
				alt=""
				loading="lazy"
				decoding="async"
				width="405"
				height="720"
				class="size-full object-cover transition-transform duration-(--motion-image) ease-ui group-hover/video:scale-[1.03]"
			/>
		{/if}
		{#if short.progress !== undefined}
			<div class="absolute inset-x-0 bottom-0 h-1 bg-white/35">
				<div
					class="h-full bg-primary"
					style:width="{Math.round(Math.min(1, Math.max(0, short.progress)) * 100)}%"
				></div>
			</div>
		{/if}
	</div>
	<div class="min-w-0">
		<svelte:element
			this={`h${headingLevel}`}
			class="line-clamp-2 text-sm leading-5 font-medium text-foreground"
		>
			<a
				href={resolve('/shorts/[id=videoid]', { id: short.id })}
				data-card-link
				title={short.title}
				onclick={onopen}
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{short.title}
			</a>
		</svelte:element>
		{#if short.viewCount !== undefined}
			<p class="mt-0.5 truncate text-xs text-muted-foreground tabular">
				{m.video_views({
					count: short.viewCount,
					formatted: formatCompactNumber(short.viewCount, locale)
				})}
			</p>
		{/if}
	</div>
</article>
