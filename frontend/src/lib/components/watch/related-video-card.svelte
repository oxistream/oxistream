<script lang="ts">
	import { resolve } from '$app/paths';
	import ChannelActionsMenu from '#lib/components/block/channel-actions-menu.svelte';
	import VideoMeta from '#lib/components/video/video-meta.svelte';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { formatDurationLong, formatPercent } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';

	let {
		video,
		now,
		actions = true,
		class: className
	}: {
		video: VideoSummary;
		/** Offers the channel actions menu, e.g. to block the channel. */
		actions?: boolean;
		/** Reference time for the relative date; fixed in stories. */
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<!-- Compact video card: a thumbnail beside the text in narrow columns, stacked once the list is wide.
     YouTube often leaves out the channel, views and date of related videos, so each is optional. -->
<article
	class={cn(
		'group/video relative flex min-w-0 gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:flex-col',
		className
	)}
>
	<VideoThumbnail
		src={video.thumbnailUrl}
		durationSeconds={video.durationSeconds}
		progress={video.progress}
		class="w-40 shrink-0 rounded-md @xl:w-full @xl:rounded-lg"
	/>
	<div class="min-w-0 flex-1 py-0.5 @xl:py-0">
		<div class="flex items-start gap-1">
			<h3 class="line-clamp-2 flex-1 text-sm leading-5 font-medium text-foreground">
				<a
					href={resolve(`/watch?v=${video.id}`)}
					data-card-link
					title={video.title}
					class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
				>
					{video.title}{#if video.durationSeconds !== undefined}<span class="sr-only">
							, {m.video_duration({ duration: formatDurationLong(video.durationSeconds, locale) })}
						</span>{/if}{#if video.progress !== undefined}<span class="sr-only">
							, {m.video_watched({ percent: formatPercent(video.progress, locale) })}
						</span>{/if}
				</a>
			</h3>
			{#if actions && video.channel.id}
				<ChannelActionsMenu
					channel={video.channel}
					videoId={video.id}
					label={m.block_video_actions({ title: video.title })}
					class="relative z-10 -me-1.5 -mt-1 shrink-0 pointer-fine:opacity-0 pointer-fine:group-hover/video:opacity-100 pointer-fine:focus-visible:opacity-100 pointer-fine:data-[state=open]:opacity-100"
				/>
			{/if}
		</div>
		{#if video.channel.id}
			<a
				href={resolve('/channel/[id]', { id: video.channel.id })}
				class="relative z-10 mt-1 block w-fit max-w-full truncate rounded-sm text-xs text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground focus-visible:outline-offset-1"
			>
				{video.channel.name}
			</a>
		{/if}
		<VideoMeta {video} {now} class="mt-0.5" />
	</div>
</article>
