<script lang="ts">
	import '#lib/sponsorblock/colors.css';
	import ChevronsRightIcon from '@lucide/svelte/icons/chevrons-right';
	import { m } from '#lib/paraglide/messages.js';
	import { categoryColor, categoryLabel } from '#lib/sponsorblock/categories.js';
	import type { SegmentCategory } from '#lib/sponsorblock/settings.js';

	let {
		skipped,
		onundo,
		skippable,
		onskip,
		label
	}: {
		/** Category just skipped automatically; shows a notice with undo. */
		skipped?: SegmentCategory;
		onundo?: () => void;
		/** Category of the segment playing now that offers a skip button. */
		skippable?: SegmentCategory;
		onskip?: () => void;
		/** Category of a label for the whole video. */
		label?: SegmentCategory;
	} = $props();

	// Taps on these controls must not reach the player's play/pause gesture.
	const stop = (event: Event) => event.stopPropagation();
</script>

<div class="oxi-segments-overlay" data-testid="segment-overlay">
	{#if label}
		<span class="oxi-segment-label" data-testid="segment-label">
			<span class="oxi-segment-dot" style:background-color={categoryColor(label)}></span>
			{categoryLabel(label)}
		</span>
	{/if}

	<div class="oxi-segment-actions">
		{#if skipped}
			<div class="oxi-segment-chip" role="status" data-testid="segment-skipped">
				<span class="oxi-segment-dot" style:background-color={categoryColor(skipped)}></span>
				<span>{m.sponsorblock_skipped({ category: categoryLabel(skipped) })}</span>
				<button
					type="button"
					class="oxi-segment-undo"
					onclick={onundo}
					onpointerdown={stop}
					onpointerup={stop}
				>
					{m.sponsorblock_undo()}
				</button>
			</div>
		{/if}
		{#if skippable}
			<button
				type="button"
				class="oxi-segment-chip oxi-segment-button"
				data-testid="segment-skip"
				onclick={onskip}
				onpointerdown={stop}
				onpointerup={stop}
			>
				<span class="oxi-segment-dot" style:background-color={categoryColor(skippable)}></span>
				{m.sponsorblock_skip({ category: categoryLabel(skippable) })}
				<ChevronsRightIcon aria-hidden="true" />
			</button>
		{/if}
	</div>
</div>

<style>
	.oxi-segments-overlay {
		position: absolute;
		inset: 0;
		/* Above the controls and their backdrop, below the centre buttons and dialogs. */
		z-index: 25;
		pointer-events: none;
		font-family: inherit;
	}

	.oxi-segment-label {
		position: absolute;
		top: 0.75rem;
		left: 0.75rem;
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.25rem 0.625rem;
		border-radius: 9999px;
		background: oklch(0.13 0.004 286 / 78%);
		color: oklch(0.99 0 0);
		font-size: 0.75rem;
		font-weight: 500;
	}

	.oxi-segment-actions {
		position: absolute;
		right: 0.75rem;
		bottom: 4.25rem;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.5rem;
	}

	.oxi-segment-chip {
		pointer-events: auto;
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.25rem;
		padding: 0.375rem 0.875rem;
		border: 1px solid oklch(1 0 0 / 14%);
		border-radius: 0.5rem;
		background: oklch(0.13 0.004 286 / 84%);
		color: oklch(0.99 0 0);
		font-size: 0.875rem;
		font-weight: 500;
		line-height: 1.25rem;
		backdrop-filter: blur(8px);
		animation: oxi-segment-in 160ms cubic-bezier(0, 0, 0.2, 1);
	}

	.oxi-segment-button {
		cursor: pointer;
		transition: background-color 120ms;
	}

	.oxi-segment-button:hover {
		background: oklch(0.22 0.006 286 / 92%);
	}

	.oxi-segment-button :global(svg) {
		width: 1rem;
		height: 1rem;
	}

	.oxi-segment-undo {
		margin-inline-start: 0.25rem;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		color: oklch(0.8 0.12 50);
		font-weight: 600;
		cursor: pointer;
	}

	.oxi-segment-undo:hover {
		background: oklch(1 0 0 / 10%);
	}

	.oxi-segment-chip:focus-visible,
	.oxi-segment-undo:focus-visible {
		outline: 2px solid oklch(0.72 0.15 50);
		outline-offset: 2px;
	}

	.oxi-segment-dot {
		width: 0.5rem;
		height: 0.5rem;
		flex-shrink: 0;
		border-radius: 9999px;
	}

	@keyframes oxi-segment-in {
		from {
			opacity: 0;
			translate: 0 0.25rem;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.oxi-segment-chip {
			animation: none;
		}
	}

	@media (width < 40rem) {
		.oxi-segment-actions {
			right: 0.5rem;
			bottom: 3.5rem;
		}

		.oxi-segment-chip {
			min-height: 2rem;
			font-size: 0.8125rem;
		}
	}
</style>
