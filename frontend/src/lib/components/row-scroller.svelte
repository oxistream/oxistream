<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import { tick, type Snippet } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		children,
		class: className
	}: {
		/** Renders the row; attach the given function to the element that scrolls. */
		children: Snippet<[Attachment<HTMLElement>]>;
		class?: string;
	} = $props();

	let root: HTMLElement | undefined;
	let row: HTMLElement | undefined;
	let prevButton = $state<HTMLButtonElement | null>(null);
	let nextButton = $state<HTMLButtonElement | null>(null);
	let canPrev = $state(false);
	let canNext = $state(false);
	let arrowTop = $state<number>();

	function update() {
		if (!row || !root) return;
		const max = row.scrollWidth - row.clientWidth;
		const prev = row.scrollLeft > 1;
		const next = row.scrollLeft < max - 1;
		// A button that disappears under the keyboard would drop focus to the page.
		const active = document.activeElement;
		const refocus =
			!prev && active === prevButton && next
				? nextButton
				: !next && active === nextButton && prev
					? prevButton
					: null;
		canPrev = prev;
		canNext = next;
		if (refocus) void tick().then(() => refocus.focus());

		const first = row.firstElementChild;
		const media = first?.querySelector('[data-thumbnail]') ?? first;
		if (media) {
			const box = media.getBoundingClientRect();
			arrowTop = box.top - root.getBoundingClientRect().top + box.height / 2;
		}
	}

	const scroller: Attachment<HTMLElement> = (node) => {
		row = node;
		// Observing reports the initial size too, so `update` never runs inside this effect.
		const resize = new ResizeObserver(update);
		const observeFirst = () => {
			if (node.firstElementChild) resize.observe(node.firstElementChild);
		};
		resize.observe(node);
		observeFirst();
		const mutation = new MutationObserver(() => {
			observeFirst();
			update();
		});
		mutation.observe(node, { childList: true });
		node.addEventListener('scroll', update, { passive: true });
		return () => {
			resize.disconnect();
			mutation.disconnect();
			node.removeEventListener('scroll', update);
			row = undefined;
			canPrev = canNext = false;
		};
	};

	function page(direction: 1 | -1) {
		if (!row) return;
		const style = getComputedStyle(row);
		const padding = parseFloat(style.paddingInlineStart) + parseFloat(style.paddingInlineEnd);
		const gap = parseFloat(style.columnGap) || 0;
		const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
		row.scrollBy({
			left: direction * (row.clientWidth - padding + gap),
			behavior: reduce ? 'auto' : 'smooth'
		});
	}

	const arrow =
		'absolute z-10 -mt-5 rounded-full bg-background shadow-md transition-[opacity,visibility,background-color] duration-(--motion-in) ease-ui hover:bg-muted dark:bg-muted dark:hover:bg-accent pointer-coarse:hidden [@media(hover:none)]:hidden';
</script>

<div bind:this={root} class={cn('relative', className)}>
	{@render children(scroller)}
	{#if arrowTop !== undefined}
		<Button
			bind:ref={prevButton}
			variant="outline"
			size="icon-lg"
			class={cn(arrow, 'start-[calc(min(1.25rem,var(--page-gutter)-0.25rem)*-1)]', {
				'invisible opacity-0': !canPrev
			})}
			style="top: {arrowTop}px"
			aria-label={m.row_previous()}
			tabindex={canPrev ? undefined : -1}
			onclick={() => page(-1)}
			data-testid="row-previous"
		>
			<ChevronLeftIcon class="size-5" />
		</Button>
		<Button
			bind:ref={nextButton}
			variant="outline"
			size="icon-lg"
			class={cn(arrow, 'end-[calc(min(1.25rem,var(--page-gutter)-0.25rem)*-1)]', {
				'invisible opacity-0': !canNext
			})}
			style="top: {arrowTop}px"
			aria-label={m.row_next()}
			tabindex={canNext ? undefined : -1}
			onclick={() => page(1)}
			data-testid="row-next"
		>
			<ChevronRightIcon class="size-5" />
		</Button>
	{/if}
</div>
