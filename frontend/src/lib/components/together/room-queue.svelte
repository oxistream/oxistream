<script lang="ts">
	import PlayIcon from '@lucide/svelte/icons/play';
	import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
	import XIcon from '@lucide/svelte/icons/x';
	import type { Snippet } from 'svelte';
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
		add,
		class: className
	}: {
		current: RoomQueueEntry | null;
		queue: RoomQueueEntry[];
		onskip?: (entry: RoomQueueEntry) => void;
		onselect?: (entry: RoomQueueEntry) => void;
		onremove?: (entry: RoomQueueEntry) => void;
		/** The form for adding videos, shown above the list. */
		add?: Snippet;
		class?: string;
	} = $props();
</script>

{#snippet entry(item: RoomQueueEntry, playing: boolean)}
	{@const video = toVideo(item.video)}
	<div class="flex min-w-0 flex-1 gap-3">
		<VideoThumbnail
			src={video.thumbnailUrl}
			durationSeconds={video.durationSeconds}
			class="w-28 shrink-0 rounded-md"
		/>
		<div class="flex min-w-0 flex-1 flex-col gap-0.5">
			<p class={cn('line-clamp-2 text-sm leading-5', playing ? 'font-semibold' : 'font-medium')}>
				{video.title}
			</p>
			<p class="truncate text-xs text-muted-foreground">{video.channel.name}</p>
			<p class="truncate text-xs text-muted-foreground">
				{m.together_added_by({ name: item.added_by })}
			</p>
		</div>
	</div>
{/snippet}

<section
	aria-labelledby="room-queue-heading"
	class={cn('flex min-h-0 flex-col rounded-xl border bg-card', className)}
	data-testid="room-queue"
>
	<header class="flex items-center justify-between gap-2 border-b px-4 py-3">
		<h2 id="room-queue-heading" class="text-sm font-semibold">{m.together_queue()}</h2>
		<span class="text-xs text-muted-foreground tabular-nums">{queue.length}</span>
	</header>
	{#if add}
		<div class="border-b p-3">{@render add()}</div>
	{/if}
	<div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
		{#if current}
			<div class="flex flex-col gap-2">
				<h3 class="px-1 text-xs font-medium text-muted-foreground">
					{m.together_now_playing()}
				</h3>
				<div class="flex items-start gap-2 rounded-lg bg-muted/60 p-2" data-testid="room-current">
					{@render entry(current, true)}
					<Button
						variant="ghost"
						size="icon-sm"
						aria-label={m.together_skip()}
						title={m.together_skip()}
						onclick={() => onskip?.(current)}
					>
						<SkipForwardIcon />
					</Button>
				</div>
			</div>
		{/if}
		<div class="flex flex-col gap-2">
			<h3 class="px-1 text-xs font-medium text-muted-foreground">{m.together_up_next()}</h3>
			{#if queue.length === 0}
				<p class="px-1 text-sm text-pretty text-muted-foreground">{m.together_queue_empty()}</p>
			{:else}
				<ol class="flex flex-col gap-1">
					{#each queue as item (item.id)}
						<li class="group/entry flex items-start gap-1 rounded-lg p-2 hover:bg-muted/60">
							{@render entry(item, false)}
							<div class="flex shrink-0 flex-col gap-1">
								<Button
									variant="ghost"
									size="icon-sm"
									aria-label={m.together_play_now()}
									title={m.together_play_now()}
									onclick={() => onselect?.(item)}
								>
									<PlayIcon />
								</Button>
								<Button
									variant="ghost"
									size="icon-sm"
									class="text-muted-foreground hover:text-destructive"
									aria-label={m.together_remove()}
									title={m.together_remove()}
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
	</div>
</section>
