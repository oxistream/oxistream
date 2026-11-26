<script lang="ts">
	import type { Snippet } from 'svelte';
	import { toChannel } from '#lib/api/mapping.js';
	import type { VideoDetails } from '#lib/api/video.js';
	import LikeButton from '#lib/components/like/like-button.svelte';
	import { cn } from '#lib/utils.js';
	import ChannelRow from './channel-row.svelte';
	import VideoDescription from './video-description.svelte';

	let {
		video,
		descriptionExpanded = $bindable(false),
		now,
		onseek,
		actions,
		class: className
	}: {
		video: VideoDetails;
		descriptionExpanded?: boolean;
		now?: Date;
		onseek?: (seconds: number) => void;
		/** Buttons beside the like button, e.g. saving to a playlist. */
		actions?: Snippet;
		class?: string;
	} = $props();

	const channel = $derived(toChannel(video.channel));
</script>

<div class={cn('flex min-w-0 flex-col gap-3', className)}>
	<h1 class="text-lg leading-snug font-semibold tracking-tight text-pretty break-words sm:text-xl">
		{video.title}
	</h1>
	<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
		<ChannelRow {channel} />
		<div class="flex flex-wrap items-center gap-2">
			<LikeButton videoId={video.id} count={video.like_count} />
			{@render actions?.()}
		</div>
	</div>
	<VideoDescription
		description={video.description}
		viewCount={video.view_count}
		viewers={video.viewers}
		publishedAt={video.published_at}
		publishedText={video.published_text}
		durationSeconds={video.duration_seconds}
		bind:expanded={descriptionExpanded}
		{now}
		{onseek}
	/>
</div>
