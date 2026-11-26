<script lang="ts">
	import ThumbsUpIcon from '@lucide/svelte/icons/thumbs-up';
	import { pickThumbnail, type VideoDetails } from '#lib/api/video.js';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import ChannelRow from './channel-row.svelte';
	import VideoDescription from './video-description.svelte';

	let {
		video,
		descriptionExpanded = $bindable(false),
		now,
		onseek,
		class: className
	}: {
		video: VideoDetails;
		descriptionExpanded?: boolean;
		now?: Date;
		onseek?: (seconds: number) => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	const channel = $derived({
		id: video.channel.id,
		name: video.channel.name,
		avatarUrl: pickThumbnail(video.channel.avatar, 80),
		subscriberCount: video.channel.subscriber_count,
		verified: video.channel.verified
	});
</script>

<div class={cn('flex min-w-0 flex-col gap-3', className)}>
	<h1 class="text-lg leading-snug font-semibold tracking-tight text-pretty break-words sm:text-xl">
		{video.title}
	</h1>
	<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
		<ChannelRow {channel} />
		{#if video.like_count != null}
			<p
				class="inline-flex h-8 items-center gap-1.5 rounded-full bg-muted px-3 text-sm font-medium tabular"
			>
				<ThumbsUpIcon class="size-4 text-muted-foreground" aria-hidden="true" />
				{m.watch_likes({
					count: video.like_count,
					formatted: formatCompactNumber(video.like_count, locale)
				})}
			</p>
		{/if}
	</div>
	<VideoDescription
		description={video.description}
		viewCount={video.view_count}
		publishedAt={video.published_at}
		publishedText={video.published_text}
		durationSeconds={video.duration_seconds}
		bind:expanded={descriptionExpanded}
		{now}
		{onseek}
	/>
</div>
