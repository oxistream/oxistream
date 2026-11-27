<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import ArrowUpToLineIcon from '@lucide/svelte/icons/arrow-up-to-line';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import PlayIcon from '@lucide/svelte/icons/play';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { dropIndex, moveItem } from '#lib/api/playlist-order.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import type { QueueEntry } from '#lib/player/queue.svelte.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();

	const uid = $props.id();
	const queue = appPlayer.queue;
	const entries = $derived(queue.entries);
	let list = $state<HTMLOListElement>();
	/** Midpoints are relative to the top of the list's scrolled content. */
	let drag = $state<{ key: string; to: number; midpoints: number[] } | null>(null);
	let announcement = $state('');

	// Another tab may change the queue mid-drag; the dragged entry is followed by key.
	const dragFrom = $derived(drag ? entries.findIndex((e) => e.key === drag!.key) : -1);
	const shown = $derived(drag && dragFrom >= 0 ? moveItem(entries, dragFrom, drag.to) : entries);

	function move(entry: QueueEntry, from: number, to: number) {
		const target = Math.max(0, Math.min(to, entries.length - 1));
		if (target === from) return;
		queue.move(from, target);
		announcement = m.playlist_moved({
			title: entry.title,
			index: target + 1,
			total: entries.length
		});
	}

	function contentY(clientY: number) {
		const rect = list!.getBoundingClientRect();
		return clientY - rect.top + list!.scrollTop;
	}

	function startDrag(event: PointerEvent, entry: QueueEntry, index: number) {
		if (!list || (event.pointerType === 'mouse' && event.button !== 0)) return;
		event.preventDefault();
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			// Synthetic pointers cannot be captured; the window listeners still follow them.
		}
		const midpoints = [...list.children].map((row) => {
			const rect = row.getBoundingClientRect();
			return contentY(rect.top + rect.height / 2);
		});
		drag = { key: entry.key, to: index, midpoints };
	}

	function dragMove(event: PointerEvent) {
		if (!drag || !list || dragFrom < 0) return;
		drag.to = dropIndex(drag.midpoints, dragFrom, contentY(event.clientY));
		const rect = list.getBoundingClientRect();
		const edge = 32;
		if (event.clientY < rect.top + edge) list.scrollBy(0, -8);
		else if (event.clientY > rect.bottom - edge) list.scrollBy(0, 8);
	}

	function endDrag() {
		if (!drag) return;
		const { key, to } = drag;
		drag = null;
		const from = entries.findIndex((e) => e.key === key);
		if (from >= 0) move(entries[from], from, to);
	}

	function handleKey(event: KeyboardEvent, entry: QueueEntry, index: number) {
		const targets: Record<string, number> = {
			ArrowUp: index - 1,
			ArrowDown: index + 1,
			Home: 0,
			End: entries.length - 1
		};
		if (!(event.key in targets)) return;
		event.preventDefault();
		move(entry, index, targets[event.key]);
		const handle = event.currentTarget as HTMLElement;
		requestAnimationFrame(() => handle.focus());
	}
</script>

<!-- On the window, since reordering moves the handle in the DOM, which can drop pointer capture. -->
<svelte:window
	onpointermove={dragMove}
	onpointerup={endDrag}
	onpointercancel={() => (drag = null)}
/>

<ol
	bind:this={list}
	class={cn('flex flex-col gap-0.5', className)}
	aria-label={m.queue_heading()}
	data-testid="queue-list"
>
	{#each shown as entry (entry.key)}
		{@const index = entries.indexOf(entry)}
		<li
			class={cn(
				'group/entry relative flex items-center gap-1.5 rounded-lg py-1.5 ps-0.5 pe-1.5 transition-colors duration-(--motion-in) ease-ui hover:bg-muted/60',
				drag?.key === entry.key && 'z-10 bg-muted shadow-md ring-1 ring-border'
			)}
			data-testid="queue-entry"
		>
			<button
				type="button"
				class="relative z-10 flex w-6 shrink-0 cursor-grab touch-none items-center justify-center self-stretch rounded-md text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring active:cursor-grabbing"
				aria-label={m.playlist_reorder_handle({ title: entry.title })}
				aria-describedby="{uid}-hint"
				onpointerdown={(event) => startDrag(event, entry, index)}
				onkeydown={(event) => handleKey(event, entry, index)}
				data-testid="queue-entry-handle"
			>
				<GripVerticalIcon class="size-4" />
			</button>
			<VideoThumbnail
				src={entry.thumbnailUrl}
				durationSeconds={entry.durationSeconds}
				class="w-20 shrink-0 rounded-md"
			/>
			<div class="min-w-0 flex-1 ps-1">
				<button
					type="button"
					class="line-clamp-2 text-start text-sm leading-5 font-medium outline-none after:absolute after:inset-0 after:rounded-lg after:content-[''] focus-visible:after:ring-2 focus-visible:after:ring-ring"
					onclick={() => appPlayer.playEntry(entry.key)}
					data-testid="queue-entry-title"
				>
					{entry.title}
				</button>
				{#if entry.channel?.name}
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
				<DropdownMenu.Content align="end" class="w-44">
					<DropdownMenu.Item onSelect={() => appPlayer.playEntry(entry.key)}>
						<PlayIcon />
						{m.queue_play_now()}
					</DropdownMenu.Item>
					<DropdownMenu.Separator />
					<DropdownMenu.Item disabled={index === 0} onSelect={() => move(entry, index, 0)}>
						<ArrowUpToLineIcon />
						{m.playlist_move_top()}
					</DropdownMenu.Item>
					<DropdownMenu.Item disabled={index === 0} onSelect={() => move(entry, index, index - 1)}>
						<ArrowUpIcon />
						{m.playlist_move_up()}
					</DropdownMenu.Item>
					<DropdownMenu.Item
						disabled={index === entries.length - 1}
						onSelect={() => move(entry, index, index + 1)}
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
<p id="{uid}-hint" class="sr-only">{m.playlist_reorder_hint()}</p>
<p class="sr-only" aria-live="polite">{announcement}</p>
