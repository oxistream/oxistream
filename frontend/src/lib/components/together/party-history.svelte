<script lang="ts">
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import PlayIcon from '@lucide/svelte/icons/play';
	import { toVideo } from '#lib/api/mapping.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		history,
		queued,
		canPlay = () => true,
		canQueue = true,
		onplay,
		onqueue,
		class: className
	}: {
		/** Played entries, newest first. */
		history: RoomQueueEntry[];
		/** Videos playing or queued, which cannot be queued again. */
		queued: Set<string>;
		/** Whether this user may play `entry` now. */
		canPlay?: (entry: RoomQueueEntry) => boolean;
		canQueue?: boolean;
		onplay: (entry: RoomQueueEntry) => void;
		onqueue: (entry: RoomQueueEntry) => void;
		class?: string;
	} = $props();
</script>

<div class={cn('flex flex-col', className)} data-testid="party-history">
	{#if history.length === 0}
		<p class="px-2 py-1.5 text-sm text-muted-foreground">{m.party_history_empty()}</p>
	{:else}
		<ol class="flex flex-col gap-0.5">
			{#each history as item (item.id)}
				{@const video = toVideo(item.video)}
				{@const isQueued = queued.has(video.id)}
				{@const playable = canPlay(item)}
				{@const rowClass = cn(
					'flex min-w-0 flex-1 items-start gap-2 rounded-md p-1.5 text-start outline-none focus-visible:ring-2 focus-visible:ring-ring',
					(playable || canQueue) && 'pe-9'
				)}
				<li
					class="group/video relative flex rounded-md transition-colors hover:bg-sidebar-accent"
					data-testid="party-history-item"
				>
					{#snippet content()}
						<VideoThumbnail
							src={video.thumbnailUrl}
							durationSeconds={video.durationSeconds}
							class="w-24 shrink-0 rounded-md"
						/>
						<span class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="line-clamp-2 text-sm leading-5 font-medium">{video.title}</span>
							<span class="truncate text-xs text-muted-foreground">{video.channel.name}</span>
							<span class="truncate text-xs text-muted-foreground">
								{item.autoplay
									? m.party_autoplay_added()
									: m.party_added_by({ name: item.added_by })}
							</span>
						</span>
					{/snippet}
					{#if playable}
						<button
							type="button"
							class={rowClass}
							title={m.party_play_now_hint()}
							onclick={() => onplay(item)}
						>
							{@render content()}
						</button>
					{:else}
						<div class={rowClass}>{@render content()}</div>
					{/if}
					{#if playable || canQueue}
						<DropdownMenu.Root>
							<DropdownMenu.Trigger
								class={cn(
									buttonVariants({ variant: 'ghost', size: 'icon-xs' }),
									'absolute end-1.5 top-1.5 text-muted-foreground hover:text-foreground data-[state=open]:bg-sidebar-accent data-[state=open]:text-foreground'
								)}
								aria-label={m.queue_item_actions({ title: video.title })}
								title={m.queue_item_actions({ title: video.title })}
								data-testid="party-history-menu"
							>
								<EllipsisVerticalIcon />
							</DropdownMenu.Trigger>
							<DropdownMenu.Content align="end" class="w-auto min-w-44">
								{#if playable}
									<DropdownMenu.Item onSelect={() => onplay(item)} data-testid="party-history-play">
										<PlayIcon />
										{m.party_play_now()}
									</DropdownMenu.Item>
								{/if}
								{#if canQueue}
									<DropdownMenu.Item
										disabled={isQueued}
										onSelect={() => onqueue(item)}
										data-testid="party-history-queue"
									>
										<ListPlusIcon />
										{isQueued ? m.party_already_queued() : m.party_queue_add()}
									</DropdownMenu.Item>
								{/if}
							</DropdownMenu.Content>
						</DropdownMenu.Root>
					{/if}
				</li>
			{/each}
		</ol>
	{/if}
</div>
