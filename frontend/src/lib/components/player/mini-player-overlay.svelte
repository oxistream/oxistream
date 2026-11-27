<script lang="ts">
	import Maximize2Icon from '@lucide/svelte/icons/maximize-2';
	import PauseIcon from '@lucide/svelte/icons/pause';
	import PlayIcon from '@lucide/svelte/icons/play';
	import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
	import XIcon from '@lucide/svelte/icons/x';
	import { MediaQuery } from 'svelte/reactivity';
	import { m } from '#lib/paraglide/messages.js';

	let {
		href,
		paused,
		ontoggle,
		onnext,
		onclose,
		closeLabel
	}: {
		/** The watch page to return to. */
		href: string;
		paused: boolean;
		/** Shows the play/pause button when set. */
		ontoggle?: () => void;
		/** Shows the next button when set. */
		onnext?: () => void;
		onclose: () => void;
		/** Defaults to closing the player. */
		closeLabel?: string;
	} = $props();

	// Without hover, a tap on the video shows the controls for a while instead of leaving.
	const hover = new MediaQuery('hover: hover');
	let revealed = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	function reveal() {
		revealed = !revealed;
		clearTimeout(timer);
		if (revealed) timer = setTimeout(() => (revealed = false), 3000);
	}

	$effect(() => () => clearTimeout(timer));

	const control =
		'pointer-events-auto grid place-items-center rounded-full text-white outline-none transition-colors duration-(--motion-in) ease-ui hover:bg-white/20 focus-visible:ring-2 focus-visible:ring-white';
</script>

<div
	class="mini-overlay"
	data-touch={hover.current ? undefined : ''}
	data-revealed={revealed ? '' : undefined}
	data-testid="mini-player-overlay"
>
	{#if hover.current}
		<!-- A click anywhere on the video leads back, like the expand button. -->
		<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by the watch page -->
		<a {href} class="absolute inset-0" tabindex="-1" aria-hidden="true"></a>
	{:else}
		<button type="button" class="absolute inset-0" tabindex="-1" aria-hidden="true" onclick={reveal}
		></button>
	{/if}
	<div class="mini-overlay-controls">
		<a
			{href}
			class="{control} absolute start-1.5 top-1.5 size-8"
			aria-label={m.mini_player_expand()}
			title={m.mini_player_expand()}
			data-testid="mini-player-expand"
		>
			<Maximize2Icon class="size-4" />
		</a>
		<!-- eslint-enable svelte/no-navigation-without-resolve -->
		<button
			type="button"
			class="{control} absolute end-1.5 top-1.5 size-8"
			aria-label={closeLabel ?? m.mini_player_close()}
			title={closeLabel ?? m.mini_player_close()}
			data-testid="mini-player-close"
			onclick={onclose}
		>
			<XIcon class="size-4.5" />
		</button>
		<div class="flex items-center gap-3">
			{#if ontoggle}
				<button
					type="button"
					class="{control} size-12 bg-black/30"
					aria-label={paused ? m.player_play() : m.player_pause()}
					data-testid="mini-player-toggle"
					onclick={ontoggle}
				>
					{#if paused}
						<PlayIcon class="size-6 fill-current" />
					{:else}
						<PauseIcon class="size-6 fill-current" />
					{/if}
				</button>
			{/if}
			{#if onnext}
				<button
					type="button"
					class="{control} size-10"
					aria-label={m.queue_next()}
					title={m.queue_next()}
					data-testid="mini-player-next"
					onclick={onnext}
				>
					<SkipForwardIcon class="size-5 fill-current" />
				</button>
			{/if}
		</div>
	</div>
</div>

<style>
	.mini-overlay {
		position: absolute;
		inset: 0;
		z-index: 1;
	}

	.mini-overlay-controls {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		pointer-events: none;
		background: linear-gradient(
			to bottom,
			oklch(0 0 0 / 45%),
			oklch(0 0 0 / 15%) 45%,
			oklch(0 0 0 / 35%)
		);
		opacity: 0;
		transition: opacity var(--motion-in, 150ms) ease-out;
	}

	.mini-overlay:hover .mini-overlay-controls,
	.mini-overlay:has(:focus-visible) .mini-overlay-controls,
	.mini-overlay[data-revealed] .mini-overlay-controls {
		opacity: 1;
	}

	/* Hidden controls must not take taps. */
	.mini-overlay[data-touch]:not([data-revealed]) .mini-overlay-controls a,
	.mini-overlay[data-touch]:not([data-revealed]) .mini-overlay-controls button {
		pointer-events: none;
	}

	@media (prefers-reduced-motion: reduce) {
		.mini-overlay-controls {
			transition: none;
		}
	}
</style>
