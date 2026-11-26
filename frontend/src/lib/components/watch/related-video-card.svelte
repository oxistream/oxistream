<script lang="ts">
	import { resolve } from '$app/paths';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import {
		formatCompactNumber,
		formatDate,
		formatDurationLong,
		formatRelativeTime
	} from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import type { RelatedVideo } from './related';

	let {
		video,
		now,
		class: className
	}: {
		video: RelatedVideo;
		/** Reference time for the relative date; fixed in stories. */
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<!-- Compact video card: a thumbnail beside the text in narrow columns, stacked once the list is wide. -->
<article
	class={cn(
		'group/video relative flex min-w-0 gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:flex-col',
		className
	)}
>
	<VideoThumbnail
		src={video.thumbnailUrl}
		durationSeconds={video.durationSeconds ?? undefined}
		class="w-40 shrink-0 rounded-md @xl:w-full @xl:rounded-lg"
	/>
	<div class="min-w-0 flex-1 py-0.5 @xl:py-0">
		<h3 class="line-clamp-2 text-sm leading-5 font-medium text-foreground">
			<a
				href={resolve(`/watch?v=${video.id}`)}
				data-card-link
				title={video.title}
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{video.title}{#if video.durationSeconds != null}<span class="sr-only">
						, {m.video_duration({ duration: formatDurationLong(video.durationSeconds, locale) })}
					</span>{/if}
			</a>
		</h3>
		{#if video.channel}
			<a
				href={resolve('/channel/[id]', { id: video.channel.id })}
				class="relative z-10 mt-1 block w-fit max-w-full truncate rounded-sm text-xs text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground focus-visible:outline-offset-1"
			>
				{video.channel.name}
			</a>
		{/if}
		{#if video.viewCount != null || video.publishedAt || video.publishedText}
			<p class="mt-0.5 truncate text-xs text-muted-foreground tabular">
				{#if video.viewCount != null}
					{m.video_views({
						count: video.viewCount,
						formatted: formatCompactNumber(video.viewCount, locale)
					})}
				{/if}
				{#if video.viewCount != null && (video.publishedAt || video.publishedText)}
					<span aria-hidden="true">·</span>
				{/if}
				{#if video.publishedAt}
					<time datetime={video.publishedAt} title={formatDate(video.publishedAt, locale)}>
						{formatRelativeTime(video.publishedAt, locale, now)}
					</time>
				{:else if video.publishedText}
					{video.publishedText}
				{/if}
			</p>
		{/if}
	</div>
</article>
