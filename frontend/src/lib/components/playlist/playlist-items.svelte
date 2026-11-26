<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowDownToLineIcon from '@lucide/svelte/icons/arrow-down-to-line';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import ArrowUpToLineIcon from '@lucide/svelte/icons/arrow-up-to-line';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { resolve } from '$app/paths';
	import { dropIndex, moveItem } from '#lib/api/playlist-order.js';
	import type { VideoSummary } from '#lib/video.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { formatDurationLong } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import { playlistWatchHref } from './links';

	let {
		playlistId,
		items,
		editable = false,
		onmove,
		onremove,
		class: className
	}: {
		playlistId: string;
		items: VideoSummary[];
		/** Shows drag handles and item menus. */
		editable?: boolean;
		onmove?: (video: VideoSummary, from: number, to: number) => void;
		onremove?: (video: VideoSummary) => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	let list = $state<HTMLOListElement>();
	let drag = $state<{ id: string; from: number; to: number; midpoints: number[] } | null>(null);
	let announcement = $state('');

	const shown = $derived(drag ? moveItem(items, drag.from, drag.to) : items);

	function move(video: VideoSummary, from: number, to: number) {
		const target = Math.max(0, Math.min(to, items.length - 1));
		if (target === from) return;
		onmove?.(video, from, target);
		announcement = m.playlist_moved({
			title: video.title,
			index: target + 1,
			total: items.length
		});
	}

	function startDrag(event: PointerEvent, video: VideoSummary, index: number) {
		if (!list || (event.pointerType === 'mouse' && event.button !== 0)) return;
		event.preventDefault();
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			// Synthetic pointers cannot be captured; the window listeners still follow them.
		}
		// Page coordinates, so scrolling during the drag does not shift the targets.
		const midpoints = [...list.children].map((row) => {
			const rect = row.getBoundingClientRect();
			return rect.top + window.scrollY + rect.height / 2;
		});
		drag = { id: video.id, from: index, to: index, midpoints };
	}

	function dragMove(event: PointerEvent) {
		if (!drag) return;
		const y = event.clientY + window.scrollY;
		drag.to = dropIndex(drag.midpoints, drag.from, y);
		const edge = 64;
		if (event.clientY < edge) window.scrollBy(0, -12);
		else if (event.clientY > window.innerHeight - edge) window.scrollBy(0, 12);
	}

	function endDrag() {
		if (!drag) return;
		const { id, from, to } = drag;
		drag = null;
		const video = items.find((item) => item.id === id);
		if (video) move(video, from, to);
	}

	function cancelDrag() {
		drag = null;
	}

	// Keyboard alternative to dragging, on the focused handle.
	function handleKey(event: KeyboardEvent, video: VideoSummary, index: number) {
		const targets: Record<string, number> = {
			ArrowUp: index - 1,
			ArrowDown: index + 1,
			Home: 0,
			End: items.length - 1
		};
		if (!(event.key in targets)) return;
		event.preventDefault();
		move(video, index, targets[event.key]);
		const handle = event.currentTarget as HTMLElement;
		requestAnimationFrame(() => handle.focus());
	}
</script>

<!-- On the window, since reordering moves the handle in the DOM, which can drop pointer capture. -->
<svelte:window onpointermove={dragMove} onpointerup={endDrag} onpointercancel={cancelDrag} />

<ol bind:this={list} class={cn('flex flex-col', className)} aria-label={m.playlist_items_label()}>
	{#each shown as video, i (video.id)}
		{@const index = items.indexOf(video)}
		{@const dragging = drag?.id === video.id}
		<li
			class={cn(
				'group/video relative flex items-center gap-2 rounded-lg py-1.5 pe-1 transition-colors duration-(--motion-in) ease-ui hover:bg-muted/60 sm:gap-3',
				editable ? 'ps-0' : 'ps-2',
				dragging && 'z-10 bg-muted shadow-md ring-1 ring-border'
			)}
			data-testid="playlist-item"
		>
			{#if editable}
				<button
					type="button"
					class="relative z-10 flex h-11 w-7 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring active:cursor-grabbing"
					aria-label={m.playlist_reorder_handle({ title: video.title })}
					aria-describedby="playlist-reorder-hint"
					onpointerdown={(event) => startDrag(event, video, index)}
					onkeydown={(event) => handleKey(event, video, index)}
				>
					<GripVerticalIcon class="size-4" />
				</button>
			{:else}
				<span class="w-6 shrink-0 text-center text-xs text-muted-foreground tabular">{i + 1}</span>
			{/if}
			<VideoThumbnail
				src={video.thumbnailUrl}
				durationSeconds={video.durationSeconds}
				class="w-32 shrink-0 rounded-md sm:w-40"
			/>
			<div class="min-w-0 flex-1">
				<h3 class="line-clamp-2 text-sm leading-5 font-medium">
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
					<a
						href={playlistWatchHref(video.id, playlistId)}
						title={video.title}
						class="rounded-sm after:absolute after:inset-0 after:rounded-lg after:content-[''] focus-visible:outline-offset-2"
					>
						{video.title}{#if video.durationSeconds !== undefined}<span class="sr-only">
								, {m.video_duration({
									duration: formatDurationLong(video.durationSeconds, locale)
								})}
							</span>{/if}
					</a>
				</h3>
				{#if video.channel.id}
					<a
						href={resolve('/channel/[id]', { id: video.channel.id })}
						class="relative z-10 mt-1 block w-fit max-w-full truncate rounded-sm text-xs text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground"
					>
						{video.channel.name}
					</a>
				{/if}
			</div>
			{#if editable}
				<DropdownMenu.Root>
					<DropdownMenu.Trigger
						class={cn(buttonVariants({ variant: 'ghost', size: 'icon' }), 'relative z-10 shrink-0')}
						aria-label={m.playlist_item_actions({ title: video.title })}
					>
						<EllipsisVerticalIcon />
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end" class="w-52">
						<DropdownMenu.Item disabled={index === 0} onSelect={() => move(video, index, 0)}>
							<ArrowUpToLineIcon />
							{m.playlist_move_top()}
						</DropdownMenu.Item>
						<DropdownMenu.Item
							disabled={index === 0}
							onSelect={() => move(video, index, index - 1)}
						>
							<ArrowUpIcon />
							{m.playlist_move_up()}
						</DropdownMenu.Item>
						<DropdownMenu.Item
							disabled={index === items.length - 1}
							onSelect={() => move(video, index, index + 1)}
						>
							<ArrowDownIcon />
							{m.playlist_move_down()}
						</DropdownMenu.Item>
						<DropdownMenu.Item
							disabled={index === items.length - 1}
							onSelect={() => move(video, index, items.length - 1)}
						>
							<ArrowDownToLineIcon />
							{m.playlist_move_bottom()}
						</DropdownMenu.Item>
						<DropdownMenu.Separator />
						<DropdownMenu.Item variant="destructive" onSelect={() => onremove?.(video)}>
							<Trash2Icon />
							{m.playlist_remove_item()}
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			{/if}
		</li>
	{/each}
</ol>
{#if editable}
	<p id="playlist-reorder-hint" class="sr-only">{m.playlist_reorder_hint()}</p>
{/if}
<p class="sr-only" aria-live="polite">{announcement}</p>
