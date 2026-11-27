<script lang="ts">
	import { resolve } from '$app/paths';
	import { toVideo } from '#lib/api/mapping.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		current,
		class: className
	}: {
		current: RoomQueueEntry | null;
		class?: string;
	} = $props();

	const video = $derived(current ? toVideo(current.video) : null);
</script>

<section
	aria-label={m.party_now_playing()}
	class={cn('flex flex-col gap-2', className)}
	data-testid="party-current"
>
	{#if current && video}
		<a href={resolve(`/watch?v=${video.id}`)} class="group/video block rounded-lg" tabindex="-1">
			<VideoThumbnail src={video.thumbnailUrl} durationSeconds={video.durationSeconds} />
		</a>
		<div class="flex min-w-0 flex-col gap-0.5 px-1">
			<a
				href={resolve(`/watch?v=${video.id}`)}
				class="line-clamp-2 text-sm leading-5 font-semibold outline-none hover:underline focus-visible:underline"
			>
				{video.title}
			</a>
			<p class="truncate text-xs text-muted-foreground">{video.channel.name}</p>
		</div>
	{:else}
		<div
			class="flex h-20 items-center justify-center rounded-lg bg-sidebar-accent text-sm text-muted-foreground"
		>
			{m.party_nothing()}
		</div>
	{/if}
</section>
