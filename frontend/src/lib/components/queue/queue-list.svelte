<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import ArrowUpToLineIcon from '@lucide/svelte/icons/arrow-up-to-line';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import PlayIcon from '@lucide/svelte/icons/play';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();

	const queue = appPlayer.queue;
	const entries = $derived(queue.entries);
</script>

<ol class={cn('flex flex-col gap-0.5', className)} data-testid="queue-list">
	{#each entries as entry, index (entry.key)}
		<li
			class="group/entry relative flex items-center gap-2.5 rounded-lg p-1.5 hover:bg-muted/60"
			data-testid="queue-entry"
		>
			<VideoThumbnail
				src={entry.thumbnailUrl}
				durationSeconds={entry.durationSeconds}
				class="w-20 shrink-0 rounded-md"
			/>
			<div class="min-w-0 flex-1">
				<button
					type="button"
					class="line-clamp-2 text-start text-sm leading-5 font-medium outline-none after:absolute after:inset-0 after:rounded-lg after:content-[''] focus-visible:after:ring-2 focus-visible:after:ring-ring"
					onclick={() => appPlayer.playEntry(entry.key)}
					data-testid="queue-entry-title"
				>
					{entry.title}
				</button>
				{#if entry.channel}
					<p class="truncate text-xs leading-4 text-muted-foreground">{entry.channel.name}</p>
				{/if}
			</div>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger
					class={cn(
						buttonVariants({ variant: 'ghost', size: 'icon-sm' }),
						'relative z-10 shrink-0 rounded-full text-muted-foreground hover:text-foreground data-[state=open]:bg-accent data-[state=open]:text-foreground'
					)}
					aria-label={m.queue_item_actions({ title: entry.title })}
					title={m.queue_item_actions({ title: entry.title })}
				>
					<EllipsisVerticalIcon />
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="end" class="w-52">
					<DropdownMenu.Item onSelect={() => appPlayer.playEntry(entry.key)}>
						<PlayIcon />
						{m.queue_play_now()}
					</DropdownMenu.Item>
					<DropdownMenu.Separator />
					<DropdownMenu.Item disabled={index === 0} onSelect={() => queue.move(index, 0)}>
						<ArrowUpToLineIcon />
						{m.playlist_move_top()}
					</DropdownMenu.Item>
					<DropdownMenu.Item disabled={index === 0} onSelect={() => queue.move(index, index - 1)}>
						<ArrowUpIcon />
						{m.playlist_move_up()}
					</DropdownMenu.Item>
					<DropdownMenu.Item
						disabled={index === entries.length - 1}
						onSelect={() => queue.move(index, index + 1)}
					>
						<ArrowDownIcon />
						{m.playlist_move_down()}
					</DropdownMenu.Item>
					<DropdownMenu.Separator />
					<DropdownMenu.Item variant="destructive" onSelect={() => queue.remove(entry.key)}>
						<Trash2Icon />
						{m.queue_remove()}
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</li>
	{/each}
</ol>
