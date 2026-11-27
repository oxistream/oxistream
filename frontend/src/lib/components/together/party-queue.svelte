<script lang="ts">
	import PlayIcon from '@lucide/svelte/icons/play';
	import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
	import XIcon from '@lucide/svelte/icons/x';
	import { resolve } from '$app/paths';
	import { toVideo } from '#lib/api/mapping.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		current,
		queue,
		onskip,
		onselect,
		onremove,
		class: className
	}: {
		current: RoomQueueEntry | null;
		queue: RoomQueueEntry[];
		onskip?: (entry: RoomQueueEntry) => void;
		onselect?: (entry: RoomQueueEntry) => void;
		onremove?: (entry: RoomQueueEntry) => void;
		class?: string;
	} = $props();
</script>

{#snippet entry(item: RoomQueueEntry, playing: boolean)}
	{@const video = toVideo(item.video)}
	<div class="flex min-w-0 flex-1 gap-3">
		<VideoThumbnail
			src={video.thumbnailUrl}
			durationSeconds={video.durationSeconds}
			class="w-24 shrink-0 rounded-md"
		/>
		<div class="flex min-w-0 flex-1 flex-col gap-0.5">
			<a
				href={resolve(`/watch?v=${video.id}`)}
				class={cn(
					'line-clamp-2 text-sm leading-5 outline-none hover:underline focus-visible:underline',
					playing ? 'font-semibold' : 'font-medium'
				)}
			>
				{video.title}
			</a>
			<p class="truncate text-xs text-muted-foreground">{video.channel.name}</p>
			<p class="truncate text-xs text-muted-foreground">
				{m.party_added_by({ name: item.added_by })}
			</p>
		</div>
	</div>
{/snippet}

<section
	aria-labelledby="party-queue-heading"
	class={cn('flex flex-col gap-2', className)}
	data-testid="party-queue"
>
	<header class="flex items-center justify-between gap-2 px-1">
		<h3 id="party-queue-heading" class="text-sm font-semibold">{m.party_queue()}</h3>
		<span class="text-xs text-muted-foreground tabular-nums">{queue.length}</span>
	</header>
	{#if current}
		<div class="flex flex-col gap-1.5">
			<h4 class="px-1 text-xs font-medium text-muted-foreground">{m.party_now_playing()}</h4>
			<div class="flex items-start gap-1 rounded-lg bg-muted/60 p-2" data-testid="party-current">
				{@render entry(current, true)}
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label={m.party_skip()}
					title={m.party_skip()}
					onclick={() => onskip?.(current)}
				>
					<SkipForwardIcon />
				</Button>
			</div>
		</div>
	{/if}
	<div class="flex flex-col gap-1.5">
		<h4 class="px-1 text-xs font-medium text-muted-foreground">{m.party_up_next()}</h4>
		{#if queue.length === 0}
			<p class="px-1 text-sm text-pretty text-muted-foreground">{m.party_queue_empty()}</p>
		{:else}
			<ol class="flex flex-col gap-0.5">
				{#each queue as item (item.id)}
					<li class="flex items-start gap-1 rounded-lg p-2 hover:bg-muted/60">
						{@render entry(item, false)}
						<div class="flex shrink-0 flex-col gap-1">
							<Button
								variant="ghost"
								size="icon-sm"
								aria-label={m.party_play_now()}
								title={m.party_play_now()}
								onclick={() => onselect?.(item)}
							>
								<PlayIcon />
							</Button>
							<Button
								variant="ghost"
								size="icon-sm"
								class="text-muted-foreground hover:text-destructive"
								aria-label={m.party_remove()}
								title={m.party_remove()}
								onclick={() => onremove?.(item)}
							>
								<XIcon />
							</Button>
						</div>
					</li>
				{/each}
			</ol>
		{/if}
	</div>
</section>
