<script lang="ts">
	import { tick } from 'svelte';
	import { pickImage } from '#lib/api/mapping.js';
	import { formatDuration } from '#lib/format.js';
	import { cn } from '#lib/utils.js';
	import type { Chapter } from './chapters';

	let {
		chapters,
		current = -1,
		onseek,
		class: className
	}: {
		chapters: Chapter[];
		/** Index of the chapter playing, -1 for none. */
		current?: number;
		/** Without it, chapters are not clickable, e.g. for party members who may not seek. */
		onseek?: (seconds: number) => void;
		class?: string;
	} = $props();

	let scroller = $state<HTMLElement>();
	let hovered = false;

	// Keeps the current chapter in view inside the list, not while the pointer is on it.
	$effect(() => {
		const index = current;
		void tick().then(() => {
			const row = scroller?.querySelector<HTMLElement>(`[data-index="${index}"]`);
			if (!scroller || !row || hovered) return;
			const top = row.offsetTop - scroller.clientHeight / 3;
			if (
				row.offsetTop < scroller.scrollTop ||
				row.offsetTop + row.offsetHeight > scroller.scrollTop + scroller.clientHeight
			) {
				scroller.scrollTo({ top, behavior: 'smooth' });
			}
		});
	});
</script>

<ol
	bind:this={scroller}
	class={cn('relative flex flex-col gap-0.5 overflow-y-auto p-1.5', className)}
	data-testid="chapter-list"
	onpointerenter={() => (hovered = true)}
	onpointerleave={() => (hovered = false)}
>
	{#each chapters as chapter, i (chapter.start)}
		{@const rowClass = cn(
			'flex w-full items-center gap-3 rounded-lg p-1.5 text-left transition-colors duration-(--motion-in) ease-ui',
			i === current && 'bg-accent'
		)}
		<li data-index={i}>
			{#if onseek}
				<button
					type="button"
					class={cn(
						rowClass,
						'cursor-pointer hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring'
					)}
					aria-current={i === current ? 'true' : undefined}
					onclick={() => onseek(chapter.start)}
					data-testid="chapter"
				>
					{@render row(chapter, i === current)}
				</button>
			{:else}
				<div
					class={rowClass}
					aria-current={i === current ? 'true' : undefined}
					data-testid="chapter"
				>
					{@render row(chapter, i === current)}
				</div>
			{/if}
		</li>
	{/each}
</ol>

{#snippet row(chapter: Chapter, active: boolean)}
	{@const thumbnail = pickImage(chapter.thumbnails, 160)}
	{#if thumbnail}
		<img
			src={thumbnail}
			alt=""
			loading="lazy"
			decoding="async"
			class="aspect-video w-20 shrink-0 rounded-md bg-muted object-cover"
		/>
	{/if}
	<span class="flex min-w-0 flex-1 flex-col gap-0.5">
		<span class="line-clamp-2 text-sm leading-snug font-medium break-words">{chapter.title}</span>
		<span class={cn('text-xs text-muted-foreground tabular', active && 'font-medium text-primary')}>
			{formatDuration(chapter.start)}
		</span>
	</span>
{/snippet}
