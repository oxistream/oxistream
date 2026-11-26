<script lang="ts">
	import { resolve } from '$app/paths';
	import ChannelActionsMenu from '#lib/components/block/channel-actions-menu.svelte';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { formatDurationLong } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import type { VideoSummary } from '#lib/video.js';
	import VideoMeta from './video-meta.svelte';
	import VideoThumbnail from './video-thumbnail.svelte';

	let {
		video,
		headingLevel = 3,
		showChannel = true,
		now,
		class: className
	}: {
		video: VideoSummary;
		headingLevel?: 2 | 3 | 4;
		showChannel?: boolean;
		/** Reference time for the relative date; fixed in stories so snapshots stay stable. */
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
	// Some listings (e.g. trending) carry videos without a known channel.
	const channelShown = $derived(showChannel && video.channel.id !== '');
	const watched = $derived(
		video.progress === undefined
			? undefined
			: new Intl.NumberFormat(locale, { style: 'percent' }).format(video.progress)
	);
</script>

<article
	class={cn(
		'group/video relative flex min-w-0 flex-col gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid',
		className
	)}
>
	<VideoThumbnail
		src={video.thumbnailUrl}
		durationSeconds={video.durationSeconds}
		progress={video.progress}
	/>
	<div class="flex gap-3">
		{#if channelShown}
			<ChannelAvatar name={video.channel.name} src={video.channel.avatarUrl} class="mt-0.5" />
		{/if}
		<div class="min-w-0 flex-1">
			<svelte:element
				this={`h${headingLevel}`}
				class="line-clamp-2 text-sm leading-5 font-medium text-foreground"
			>
				<a
					href={resolve(`/watch?v=${video.id}`)}
					data-card-link
					title={video.title}
					class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
				>
					{video.title}<span class="sr-only">
						{#if video.durationSeconds !== undefined}, {m.video_duration({
								duration: formatDurationLong(video.durationSeconds, locale)
							})}{/if}{#if watched}, {m.video_watched({ percent: watched })}{/if}
					</span>
				</a>
			</svelte:element>
			{#if channelShown}
				<a
					href={resolve('/channel/[id]', { id: video.channel.id })}
					class="relative z-10 mt-1 block w-fit max-w-full truncate rounded-sm text-xs text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground focus-visible:outline-offset-1"
				>
					{video.channel.name}
				</a>
			{/if}
			<VideoMeta {video} {now} />
		</div>
		{#if channelShown}
			<ChannelActionsMenu
				channel={video.channel}
				label={m.block_video_actions({ title: video.title })}
				class="relative z-10 -me-1.5 shrink-0 pointer-fine:opacity-0 pointer-fine:group-hover/video:opacity-100 pointer-fine:focus-visible:opacity-100 pointer-fine:data-[state=open]:opacity-100"
			/>
		{/if}
	</div>
</article>
