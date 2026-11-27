<script lang="ts">
	import PlayIcon from '@lucide/svelte/icons/play';
	import XIcon from '@lucide/svelte/icons/x';
	import { resolve } from '$app/paths';
	import { toVideo } from '#lib/api/mapping.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		queue,
		onselect,
		onremove,
		class: className
	}: {
		queue: RoomQueueEntry[];
		onselect?: (entry: RoomQueueEntry) => void;
		onremove?: (entry: RoomQueueEntry) => void;
		class?: string;
	} = $props();
</script>

<div class={cn('flex flex-col', className)} data-testid="party-queue">
	{#if queue.length === 0}
		<p class="px-2 py-1.5 text-sm text-muted-foreground">{m.party_queue_empty()}</p>
	{:else}
		<ol class="flex flex-col gap-0.5">
			{#each queue as item (item.id)}
				{@const video = toVideo(item.video)}
				<li
					class="group/video flex items-start gap-2 rounded-md p-1.5 transition-colors hover:bg-sidebar-accent"
				>
					<VideoThumbnail
						src={video.thumbnailUrl}
						durationSeconds={video.durationSeconds}
						class="w-24 shrink-0 rounded-md"
					/>
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<a
							href={resolve(`/watch?v=${video.id}`)}
							class="line-clamp-2 text-sm leading-5 font-medium outline-none hover:underline focus-visible:underline"
						>
							{video.title}
						</a>
						<p class="truncate text-xs text-muted-foreground">{video.channel.name}</p>
						<p class="truncate text-xs text-muted-foreground">
							{m.party_added_by({ name: item.added_by })}
						</p>
					</div>
					<div class="flex shrink-0 flex-col">
						<Button
							variant="ghost"
							size="icon-xs"
							aria-label={m.party_play_now()}
							title={m.party_play_now()}
							onclick={() => onselect?.(item)}
						>
							<PlayIcon />
						</Button>
						<Button
							variant="ghost"
							size="icon-xs"
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
