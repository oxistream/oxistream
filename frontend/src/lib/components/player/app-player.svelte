<script lang="ts">
	import { page } from '$app/state';
	import { createProgressReporter } from '#lib/api/history.js';
	import { useSegments, useSponsorBlockSettings } from '#lib/api/sponsorblock.svelte.js';
	import { manifestUrl } from '#lib/api/video.js';
	import SponsorBlockDialog from '#lib/components/sponsorblock/sponsorblock-dialog.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer, type PlayerMedia } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';
	import { resolveSegments } from '#lib/sponsorblock/skip.js';
	import MiniPlayerBar from './mini-player-bar.svelte';
	import Player from './player.svelte';

	let { media }: { media: PlayerMedia } = $props();

	const reportProgress = createProgressReporter();
	const segmentsQuery = useSegments(() => media.videoId);
	const sponsorBlock = useSponsorBlockSettings();
	const segments = $derived.by(() => {
		if (!sponsorBlock.ready || !segmentsQuery.data) return [];
		const resolved = resolveSegments(segmentsQuery.data, sponsorBlock.current);
		if (media.autoSkip !== false) return resolved;
		return resolved.map((s) => (s.mode === 'auto' ? { ...s, mode: 'button' as const } : s));
	});
	let sponsorBlockOpen = $state(false);

	async function openSponsorBlock() {
		// The dialog cannot show above a fullscreen player.
		if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
		sponsorBlockOpen = true;
	}

	// Shorts bring their own player, so this one waits out of sight, unless a party holds it.
	const elsewhere = $derived(!appPlayer.claim && page.route.id?.startsWith('/shorts') === true);
	const mode = $derived(appPlayer.inline ? 'inline' : elsewhere ? 'hidden' : 'mini');
	const options = $derived(appPlayer.inline ? appPlayer.slotOptions() : {});

	$effect(() => {
		if (elsewhere) appPlayer.pause();
	});

	let host = $state<HTMLElement>();
	$effect(() => {
		const el = host ?? null;
		appPlayer.element = el;
		return () => {
			if (appPlayer.element === el) appPlayer.element = null;
		};
	});
	let rect = $state<{ top: number; left: number; width: number } | null>(null);
	// The slot asks for square corners where the player runs edge to edge.
	let flush = $state(false);

	$effect(() => {
		const slot = appPlayer.slotElement;
		const parent = host?.parentElement;
		if (!slot || !parent) {
			rect = null;
			return;
		}
		const measure = () => {
			const s = slot.getBoundingClientRect();
			const p = parent.getBoundingClientRect();
			rect = { top: s.top - p.top, left: s.left - p.left, width: s.width };
			flush = getComputedStyle(slot).getPropertyValue('--oxi-slot-flush').trim() === '1';
		};
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(slot);
		observer.observe(parent);
		window.addEventListener('resize', measure);
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', measure);
		};
	});

	let instance = $state<ReturnType<typeof Player>>();
	$effect(() => {
		appPlayer.controls = instance;
	});

	function ended() {
		appPlayer.paused = true;
		media.events?.onended?.();
		if (appPlayer.inline) appPlayer.slotOptions().onended?.();
	}
</script>

<SponsorBlockDialog bind:open={sponsorBlockOpen} />

<div
	bind:this={host}
	class="app-player"
	data-mode={mode}
	data-flush={mode === 'inline' && flush ? '' : undefined}
	data-measured={rect ? '' : undefined}
	data-testid={mode === 'mini' ? 'mini-player' : 'app-player'}
	role={mode === 'mini' ? 'region' : undefined}
	aria-label={mode === 'mini' ? m.mini_player_label() : undefined}
	style:top={mode === 'inline' && rect ? `${rect.top}px` : undefined}
	style:left={mode === 'inline' && rect ? `${rect.left}px` : undefined}
	style:width={mode === 'inline' && rect ? `${rect.width}px` : undefined}
	style:--app-player-end-inset={appPlayer.endInset}
>
	<div class="app-player-video">
		{#key appPlayer.generation}
			<Player
				bind:this={instance}
				src={manifestUrl(media.videoId)}
				poster={media.poster}
				subtitles={media.subtitles}
				storyboard={media.storyboard}
				startTime={media.startTime ?? 0}
				playback={media.playback}
				theater={options.theater ?? false}
				globalHotkeys={mode === 'inline'}
				autoplay={media.autoplay ?? true}
				{segments}
				externalRate={media.externalRate}
				onsponsorblocksettings={session.sponsorblock ? openSponsorBlock : undefined}
				ontheatertoggle={options.ontheatertoggle}
				onprogress={media.trackProgress === false ? undefined : reportProgress(media.videoId)}
				onplay={(time) => {
					appPlayer.paused = false;
					media.events?.onplay?.(time);
				}}
				onpause={(time) => {
					appPlayer.paused = true;
					media.events?.onpause?.(time);
				}}
				onseeked={media.events?.onseeked}
				ontimeupdate={media.events?.ontimeupdate}
				onended={ended}
			/>
		{/key}
		{@render appPlayer.claim?.overlay?.()}
		{#if mode === 'mini'}
			<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by the watch page -->
			<a
				href={media.href}
				class="app-player-expand"
				aria-label={m.mini_player_expand()}
				data-testid="mini-player-expand"
			></a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
		{/if}
	</div>
	{#if mode === 'mini'}
		<MiniPlayerBar
			title={media.title}
			channel={media.channel?.name}
			href={media.href}
			paused={appPlayer.paused}
			ontoggle={() => appPlayer.toggle()}
			closeLabel={appPlayer.claim?.closeLabel?.()}
			onclose={() => appPlayer.close()}
			class="app-player-bar"
		/>
	{/if}
</div>

<style>
	.app-player[data-mode='inline'] {
		position: absolute;
		z-index: 10;
	}

	/* Until the slot is measured, e.g. on the first frame. */
	.app-player[data-mode='inline']:not([data-measured]) {
		visibility: hidden;
	}

	.app-player[data-mode='hidden'] {
		display: none;
	}

	.app-player[data-flush] :global(.oxi-player-frame) {
		--media-video-border-radius: 0;
	}

	.app-player[data-flush] :global(.oxi-player-frame)::after {
		border-color: transparent;
	}

	.app-player[data-mode='mini'] {
		position: fixed;
		inset-inline-end: calc(1rem + var(--app-player-end-inset, 0px));
		bottom: calc(1rem + env(safe-area-inset-bottom));
		z-index: 40;
		transition: inset-inline-end 200ms ease-in-out;
		width: 22rem;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-xl);
		background: var(--card);
		color: var(--card-foreground);
		box-shadow:
			0 1px 2px oklch(0 0 0 / 6%),
			0 12px 32px -8px oklch(0 0 0 / 24%);
	}

	.app-player-video {
		position: relative;
	}

	.app-player[data-mode='mini'] :global(.oxi-player-frame) {
		--media-video-border-radius: 0;
	}

	.app-player[data-mode='mini'] :global(.oxi-player-frame)::after {
		border-color: transparent;
	}

	/* The mini player is a preview: no controls on the video, a click returns to the watch page. */
	.app-player[data-mode='mini'] :global(media-controls),
	.app-player[data-mode='mini'] :global(.video-status-indicators),
	.app-player[data-mode='mini'] :global(.oxi-segments-overlay),
	.app-player[data-mode='mini'] :global(media-title) {
		display: none;
	}

	.app-player-expand {
		position: absolute;
		inset: 0;
		z-index: 1;
		outline: none;
	}

	.app-player-expand:focus-visible {
		box-shadow: inset 0 0 0 2px var(--ring);
	}

	/* Phones: a slim card along the bottom edge. */
	@media (width < 40rem) {
		.app-player[data-mode='mini'] {
			inset-inline: 0.5rem;
			bottom: calc(0.5rem + env(safe-area-inset-bottom));
			display: flex;
			align-items: center;
			width: auto;
			border-radius: var(--radius-lg);
		}

		.app-player[data-mode='mini'] .app-player-video {
			width: 7rem;
			flex-shrink: 0;
		}

		.app-player[data-mode='mini'] :global(.app-player-bar) {
			flex: 1;
		}
	}
</style>
