<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';
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
				<li class="group/video relative rounded-md transition-colors hover:bg-sidebar-accent">
					<button
						type="button"
						class="flex w-full items-start gap-2 rounded-md p-1.5 pe-8 text-start outline-none focus-visible:ring-2 focus-visible:ring-ring"
						title={m.party_play_now_hint()}
						onclick={() => onselect?.(item)}
					>
						<VideoThumbnail
							src={video.thumbnailUrl}
							durationSeconds={video.durationSeconds}
							class="w-24 shrink-0 rounded-md"
						/>
						<span class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="line-clamp-2 text-sm leading-5 font-medium">{video.title}</span>
							<span class="truncate text-xs text-muted-foreground">{video.channel.name}</span>
							<span class="truncate text-xs text-muted-foreground">
								{m.party_added_by({ name: item.added_by })}
							</span>
						</span>
					</button>
					<Button
						variant="ghost"
						size="icon-xs"
						class="absolute end-1.5 top-1.5 text-muted-foreground hover:text-destructive"
						aria-label={m.party_remove()}
						title={m.party_remove()}
						onclick={() => onremove?.(item)}
					>
						<XIcon />
					</Button>
				</li>
			{/each}
		</ol>
	{/if}
</div>
