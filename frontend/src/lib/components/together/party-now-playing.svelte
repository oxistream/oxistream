<script lang="ts">
	import PauseIcon from '@lucide/svelte/icons/pause';
	import PlayIcon from '@lucide/svelte/icons/play';
	import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
	import { resolve } from '$app/paths';
	import { toVideo } from '#lib/api/mapping.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		current,
		playing,
		onskip,
		class: className
	}: {
		current: RoomQueueEntry | null;
		playing: boolean;
		onskip?: (entry: RoomQueueEntry) => void;
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
		<div class="flex items-start gap-1">
			<div class="flex min-w-0 flex-1 flex-col gap-0.5 px-1">
				<a
					href={resolve(`/watch?v=${video.id}`)}
					class="line-clamp-2 text-sm leading-5 font-semibold outline-none hover:underline focus-visible:underline"
				>
					{video.title}
				</a>
				<p class="truncate text-xs text-muted-foreground">{video.channel.name}</p>
				<p
					class="flex items-center gap-1 text-xs font-medium text-muted-foreground"
					data-testid="party-playback"
					data-playing={playing}
				>
					{#if playing}
						<PlayIcon class="size-3 fill-current text-primary" aria-hidden="true" />
						{m.party_playing()}
					{:else}
						<PauseIcon class="size-3 fill-current" aria-hidden="true" />
						{m.party_paused()}
					{/if}
				</p>
			</div>
			<Button
				variant="ghost"
				size="icon-sm"
				class="shrink-0"
				aria-label={m.party_skip()}
				title={m.party_skip()}
				onclick={() => onskip?.(current)}
			>
				<SkipForwardIcon />
			</Button>
		</div>
	{:else}
		<div
			class="flex h-20 items-center justify-center rounded-lg bg-sidebar-accent text-sm text-muted-foreground"
		>
			{m.party_nothing()}
		</div>
	{/if}
</section>
