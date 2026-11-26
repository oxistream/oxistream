<script lang="ts" generics="T extends { id: string; label: string }">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import type { Snippet } from 'svelte';
	import { dropIndex, moveItem } from '#lib/api/playlist-order.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		items,
		label,
		onmove,
		trailing,
		disabled = false,
		class: className
	}: {
		items: T[];
		/** Accessible name of the list. */
		label: string;
		onmove: (from: number, to: number) => void;
		/** Extra controls per item, e.g. a toggle. */
		trailing?: Snippet<[T, number]>;
		disabled?: boolean;
		class?: string;
	} = $props();

	const uid = $props.id();
	let list = $state<HTMLOListElement>();
	let drag = $state<{ id: string; from: number; to: number; midpoints: number[] } | null>(null);
	let announcement = $state('');

	const shown = $derived(drag ? moveItem(items, drag.from, drag.to) : items);

	function move(item: T, from: number, to: number) {
		const target = Math.max(0, Math.min(to, items.length - 1));
		if (target === from) return;
		onmove(from, target);
		announcement = m.settings_moved({ label: item.label, index: target + 1, total: items.length });
	}

	function startDrag(event: PointerEvent, item: T, index: number) {
		if (disabled || !list || (event.pointerType === 'mouse' && event.button !== 0)) return;
		event.preventDefault();
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			// Synthetic pointers cannot be captured; the window listeners still follow them.
		}
		const midpoints = [...list.children].map((row) => {
			const rect = row.getBoundingClientRect();
			return rect.top + window.scrollY + rect.height / 2;
		});
		drag = { id: item.id, from: index, to: index, midpoints };
	}

	function dragMove(event: PointerEvent) {
		if (!drag) return;
		drag.to = dropIndex(drag.midpoints, drag.from, event.clientY + window.scrollY);
	}

	function endDrag() {
		if (!drag) return;
		const { id, from, to } = drag;
		drag = null;
		const item = items.find((i) => i.id === id);
		if (item) move(item, from, to);
	}

	function handleKey(event: KeyboardEvent, item: T, index: number) {
		const targets: Record<string, number> = {
			ArrowUp: index - 1,
			ArrowDown: index + 1,
			Home: 0,
			End: items.length - 1
		};
		if (disabled || !(event.key in targets)) return;
		event.preventDefault();
		move(item, index, targets[event.key]);
		const handle = event.currentTarget as HTMLElement;
		requestAnimationFrame(() => handle.focus());
	}
</script>

<svelte:window
	onpointermove={dragMove}
	onpointerup={endDrag}
	onpointercancel={() => (drag = null)}
/>

<ol bind:this={list} class={cn('flex flex-col gap-1', className)} aria-label={label}>
	{#each shown as item (item.id)}
		{@const index = items.indexOf(item)}
		<li
			class={cn(
				'flex items-center gap-2 rounded-lg border bg-background py-1 ps-1 pe-2 transition-shadow duration-(--motion-in) ease-ui',
				drag?.id === item.id && 'relative z-10 shadow-md ring-1 ring-ring/40'
			)}
			data-testid="reorder-item"
		>
			<button
				type="button"
				class="flex h-9 w-7 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring active:cursor-grabbing disabled:cursor-not-allowed disabled:opacity-50"
				aria-label={m.settings_reorder_handle({ label: item.label })}
				aria-describedby="{uid}-hint"
				{disabled}
				onpointerdown={(event) => startDrag(event, item, index)}
				onkeydown={(event) => handleKey(event, item, index)}
			>
				<GripVerticalIcon class="size-4" />
			</button>
			<span class="hidden w-5 shrink-0 text-center text-xs text-muted-foreground tabular sm:inline">
				{index + 1}
			</span>
			<span class="min-w-0 flex-1 py-1 text-sm leading-5 text-pretty">{item.label}</span>
			<div class="flex shrink-0 items-center">
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label={m.settings_move_up({ label: item.label })}
					disabled={disabled || index === 0}
					onclick={() => move(item, index, index - 1)}
				>
					<ArrowUpIcon />
				</Button>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label={m.settings_move_down({ label: item.label })}
					disabled={disabled || index === items.length - 1}
					onclick={() => move(item, index, index + 1)}
				>
					<ArrowDownIcon />
				</Button>
			</div>
			{#if trailing}
				<div class="flex shrink-0 items-center ps-1">{@render trailing(item, index)}</div>
			{/if}
		</li>
	{/each}
</ol>
<p id="{uid}-hint" class="sr-only">{m.playlist_reorder_hint()}</p>
<p class="sr-only" aria-live="polite">{announcement}</p>
