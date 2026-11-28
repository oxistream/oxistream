<script lang="ts">
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import XIcon from '@lucide/svelte/icons/x';
	import { toVideo } from '#lib/api/mapping.js';
	import { dropIndex, moveItem } from '#lib/api/playlist-order.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	/** How long a reorder shows before the room confirms it. */
	const PENDING_MS = 3000;

	let {
		queue,
		revision = 0,
		onselect,
		onremove,
		canRemove = () => true,
		onmove,
		class: className
	}: {
		queue: RoomQueueEntry[];
		/** Of the room `queue` belongs to; a newer one replaces a pending reorder. */
		revision?: number;
		/** Makes entries clickable, to play them now. */
		onselect?: (entry: RoomQueueEntry) => void;
		onremove?: (entry: RoomQueueEntry) => void;
		/** Whether `entry` shows a remove button. */
		canRemove?: (entry: RoomQueueEntry) => boolean;
		/** Shows drag handles; `to` is the new index in `queue`. */
		onmove?: (entry: RoomQueueEntry, to: number) => void;
		class?: string;
	} = $props();

	const uid = $props.id();
	let list = $state<HTMLOListElement>();
	let drag = $state<{ id: number; from: number; to: number; midpoints: number[] } | null>(null);
	let pending = $state<{ ids: number[]; revision: number } | null>(null);
	let announcement = $state('');

	const ordered = $derived.by(() => {
		if (!pending || pending.revision !== revision) return queue;
		const byId = new Map(queue.map((entry) => [entry.id, entry]));
		const entries = pending.ids.flatMap((id) => byId.get(id) ?? []);
		return entries.length === queue.length ? entries : queue;
	});
	const shown = $derived(drag ? moveItem(ordered, drag.from, drag.to) : ordered);

	$effect(() => {
		if (!pending) return;
		const timer = setTimeout(() => (pending = null), PENDING_MS);
		return () => clearTimeout(timer);
	});

	function move(entry: RoomQueueEntry, from: number, to: number) {
		const target = Math.max(0, Math.min(to, ordered.length - 1));
		if (target === from) return;
		pending = { ids: moveItem(ordered, from, target).map((e) => e.id), revision };
		onmove?.(entry, target);
		announcement = m.playlist_moved({
			title: entry.video.title,
			index: target + 1,
			total: ordered.length
		});
	}

	/** The element that scrolls the list: the panel body. */
	function scroller(): HTMLElement | null {
		let node = list?.parentElement ?? null;
		while (node) {
			if (/(auto|scroll)/.test(getComputedStyle(node).overflowY)) return node;
			node = node.parentElement;
		}
		return null;
	}

	function startDrag(event: PointerEvent, entry: RoomQueueEntry, index: number) {
		if (!list || (event.pointerType === 'mouse' && event.button !== 0)) return;
		event.preventDefault();
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			// Synthetic pointers cannot be captured; the window listeners still follow them.
		}
		// Relative to the list, so scrolling the panel during the drag does not shift the targets.
		const top = list.getBoundingClientRect().top;
		const midpoints = [...list.children].map((row) => {
			const rect = row.getBoundingClientRect();
			return rect.top - top + rect.height / 2;
		});
		drag = { id: entry.id, from: index, to: index, midpoints };
	}

	function dragMove(event: PointerEvent) {
		if (!drag || !list) return;
		drag.to = dropIndex(
			drag.midpoints,
			drag.from,
			event.clientY - list.getBoundingClientRect().top
		);
		const area = scroller();
		if (!area) return;
		const rect = area.getBoundingClientRect();
		const edge = 48;
		if (event.clientY < rect.top + edge) area.scrollBy(0, -12);
		else if (event.clientY > rect.bottom - edge) area.scrollBy(0, 12);
	}

	function endDrag() {
		if (!drag) return;
		const { id, from, to } = drag;
		drag = null;
		const entry = ordered.find((e) => e.id === id);
		if (entry) move(entry, from, to);
	}

	// Keyboard alternative to dragging, on the focused handle.
	function handleKey(event: KeyboardEvent, entry: RoomQueueEntry, index: number) {
		const targets: Record<string, number> = {
			ArrowUp: index - 1,
			ArrowDown: index + 1,
			Home: 0,
			End: ordered.length - 1
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

<div class={cn('flex flex-col', className)} data-testid="party-queue">
	{#if queue.length === 0}
		<p class="px-2 py-1.5 text-sm text-muted-foreground">{m.party_queue_empty()}</p>
	{:else}
		<ol bind:this={list} class="flex flex-col gap-0.5">
			{#each shown as item (item.id)}
				{@const video = toVideo(item.video)}
				{@const index = ordered.indexOf(item)}
				{@const removable = onremove !== undefined && canRemove(item)}
				{@const rowClass = cn(
					'flex min-w-0 flex-1 items-start gap-2 rounded-md p-1.5 text-start outline-none focus-visible:ring-2 focus-visible:ring-ring',
					onmove && 'ps-0.5',
					removable && 'pe-8'
				)}
				<li
					class={cn(
						'group/video relative flex rounded-md transition-colors hover:bg-sidebar-accent',
						drag?.id === item.id && 'z-10 bg-sidebar-accent shadow-md ring-1 ring-border'
					)}
					data-testid="party-queue-item"
				>
					{#if onmove}
						<button
							type="button"
							class="flex w-6 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing"
							aria-label={m.playlist_reorder_handle({ title: video.title })}
							aria-describedby="{uid}-hint"
							onpointerdown={(event) => startDrag(event, item, index)}
							onkeydown={(event) => handleKey(event, item, index)}
							data-testid="party-queue-handle"
						>
							<GripVerticalIcon class="size-4" />
						</button>
					{/if}
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
					{#if onselect}
						<button
							type="button"
							class={rowClass}
							title={m.party_play_now_hint()}
							onclick={() => onselect(item)}
						>
							{@render content()}
						</button>
					{:else}
						<div class={rowClass}>{@render content()}</div>
					{/if}
					{#if removable}
						<Button
							variant="ghost"
							size="icon-xs"
							class="absolute end-1.5 top-1.5 text-muted-foreground hover:text-destructive"
							aria-label={m.party_remove()}
							title={m.party_remove()}
							onclick={() => onremove?.(item)}
							data-testid="party-queue-remove"
						>
							<XIcon />
						</Button>
					{/if}
				</li>
			{/each}
		</ol>
		{#if onmove}
			<p id="{uid}-hint" class="sr-only">{m.playlist_reorder_hint()}</p>
		{/if}
	{/if}
	<p class="sr-only" aria-live="polite">{announcement}</p>
</div>
