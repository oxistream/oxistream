<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { VideoSummary } from '#lib/video.js';

	let {
		video,
		queued,
		onpick
	}: {
		video: VideoSummary;
		queued: boolean;
		onpick: (video: VideoSummary) => void;
	} = $props();
</script>

<!-- A video that can be queued with one click; marked once it is in the queue. -->
<button
	type="button"
	class="group/pick flex w-full items-center gap-2 rounded-md p-1 text-start outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default disabled:hover:bg-transparent"
	disabled={queued}
	aria-label={queued
		? `${video.title}, ${m.party_add_queued()}`
		: m.party_add_video({ title: video.title })}
	onclick={() => onpick(video)}
	data-testid="party-pick"
	data-queued={queued}
>
	<VideoThumbnail
		src={video.thumbnailUrl}
		durationSeconds={video.durationSeconds}
		class="w-20 shrink-0 rounded-md"
	/>
	<span class="flex min-w-0 flex-1 flex-col">
		<span class="line-clamp-2 text-xs leading-4 font-medium">{video.title}</span>
		<span class="truncate text-xs text-muted-foreground">{video.channel.name}</span>
	</span>
	<span
		class="flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground group-hover/pick:text-foreground group-disabled/pick:text-primary"
		aria-hidden="true"
	>
		{#if queued}
			<CheckIcon class="size-4" />
		{:else}
			<PlusIcon class="size-4" />
		{/if}
	</span>
</button>
