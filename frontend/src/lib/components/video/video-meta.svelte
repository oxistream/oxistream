<script lang="ts">
	import { formatCompactNumber, formatDate, formatRelativeTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';

	let {
		video,
		now,
		class: className
	}: {
		video: Pick<VideoSummary, 'viewCount' | 'viewers' | 'publishedAt' | 'publishedText'>;
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
	const hasDate = $derived(Boolean(video.publishedAt || video.publishedText));
	const hasCount = $derived(video.viewCount !== undefined || video.viewers !== undefined);
</script>

{#if hasCount || hasDate}
	<p class={cn('truncate text-xs text-muted-foreground tabular', className)}>
		{#if video.viewers !== undefined}
			{m.video_watching({
				count: video.viewers,
				formatted: formatCompactNumber(video.viewers, locale)
			})}
		{:else if video.viewCount !== undefined}
			{m.video_views({
				count: video.viewCount,
				formatted: formatCompactNumber(video.viewCount, locale)
			})}
		{/if}
		{#if hasCount && hasDate}
			<span aria-hidden="true">·</span>
		{/if}
		{#if video.publishedAt}
			<time datetime={video.publishedAt} title={formatDate(video.publishedAt, locale)}>
				{formatRelativeTime(video.publishedAt, locale, now)}
			</time>
		{:else if video.publishedText}
			<span>{video.publishedText}</span>
		{/if}
	</p>
{/if}
