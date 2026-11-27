<script lang="ts">
	import { resolve } from '$app/paths';
	import ChannelActionsMenu from '#lib/components/block/channel-actions-menu.svelte';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
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
	const live = $derived(video.isLive === true);
</script>

<!-- Stacks below the container's xl width; side by side above it. Needs an @container ancestor. -->
<article
	class={cn(
		'group/video relative grid min-w-0 gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:grid-cols-[clamp(15rem,38%,22.5rem)_minmax(0,1fr)] @xl:gap-4',
		className
	)}
>
	<VideoThumbnail
		src={video.thumbnailUrl}
		durationSeconds={video.durationSeconds}
		progress={video.progress}
		badge={live ? m.search_live() : undefined}
		unavailable={live}
	/>
	<div class="flex min-w-0 flex-col gap-1 @xl:pt-0.5">
		<div class="flex items-start gap-2">
			<h3
				class={cn(
					'line-clamp-2 flex-1 text-sm leading-5 font-medium @xl:text-base @xl:leading-6',
					live ? 'text-muted-foreground' : 'text-foreground'
				)}
			>
				{#if live}
					<!-- Livestreams are not played yet, so they get no watch link. -->
					<span title={video.title} data-testid="live-result">
						{video.title}<span class="sr-only">, {m.search_live_unavailable()}</span>
					</span>
				{:else}
					<a
						href={resolve(`/watch?v=${video.id}`)}
						data-card-link
						title={video.title}
						class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
					>
						{video.title}{#if video.durationSeconds !== undefined}<span class="sr-only">
								, {m.video_duration({
									duration: formatDurationLong(video.durationSeconds, locale)
								})}
							</span>{/if}{#if video.progress !== undefined}<span class="sr-only">
								, {m.video_watched({ percent: formatPercent(video.progress, locale) })}
							</span>{/if}
					</a>
				{/if}
			</h3>
			{#if actions && video.channel.id}
				<ChannelActionsMenu
					channel={video.channel}
					{video}
					label={m.block_video_actions({ title: video.title })}
					class="relative z-10 -me-1.5 -mt-1 shrink-0 pointer-fine:opacity-0 pointer-fine:group-hover/video:opacity-100 pointer-fine:focus-visible:opacity-100 pointer-fine:data-[state=open]:opacity-100"
				/>
			{/if}
		</div>
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
