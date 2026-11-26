<script lang="ts">
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
	import VideoMeta from '#lib/components/video/video-meta.svelte';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { formatDurationLong } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';

	let {
		video,
		now,
		class: className
	}: {
		video: VideoSummary;
		/** Reference time for the relative date; fixed in stories. */
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<!-- Stacks below the container's xl width; side by side above it. Needs an @container ancestor. -->
<article
	class={cn(
		'group/video relative grid min-w-0 gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] @xl:gap-4 @4xl:grid-cols-[22rem_minmax(0,1fr)]',
		className
	)}
>
	<VideoThumbnail
		src={video.thumbnailUrl}
		durationSeconds={video.durationSeconds}
		badge={video.isLive ? m.search_live() : undefined}
	/>
	<div class="flex min-w-0 flex-col gap-1 @xl:pt-0.5">
		<h3
			class="line-clamp-2 text-sm leading-5 font-medium text-foreground @xl:text-base @xl:leading-6"
		>
			<a
				href={resolve(`/watch?v=${video.id}`)}
				data-card-link
				title={video.title}
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{video.title}{#if video.durationSeconds !== undefined}<span class="sr-only">
						, {m.video_duration({ duration: formatDurationLong(video.durationSeconds, locale) })}
					</span>{/if}
			</a>
		</h3>
		<VideoMeta {video} {now} />
		{#if video.channel.id}
			<a
				href={resolve('/channel/[id]', { id: video.channel.id })}
				class="relative z-10 flex w-fit max-w-full items-center gap-2 rounded-sm py-1 text-xs text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground focus-visible:outline-offset-1 @xl:py-1.5"
			>
				<ChannelAvatar name={video.channel.name} src={video.channel.avatarUrl} size="sm" />
				<span class="truncate">{video.channel.name}</span>
				{#if video.channel.verified}<VerifiedBadge class="text-sm" />{/if}
			</a>
		{/if}
		{#if video.description}
			<p class="hidden text-xs text-pretty text-muted-foreground @xl:line-clamp-2">
				{video.description}
			</p>
		{/if}
	</div>
</article>
