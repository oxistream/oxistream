<script lang="ts">
	import { MediaQuery } from 'svelte/reactivity';
	import { page } from '$app/state';
	import { createProgressReporter } from '#lib/api/history.js';
	import { useSegments, useSponsorBlockSettings } from '#lib/api/sponsorblock.svelte.js';
	import { manifestUrl } from '#lib/api/video.js';
	import SponsorBlockDialog from '#lib/components/sponsorblock/sponsorblock-dialog.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer, type PlayerMedia } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';
	import { resolveSegments } from '#lib/sponsorblock/skip.js';
	import PlayerMessage from '#lib/components/watch/player-message.svelte';
	import UpNext from '#lib/components/queue/up-next.svelte';
	import MiniPlayerBar from './mini-player-bar.svelte';
	import MiniPlayerOverlay from './mini-player-overlay.svelte';
	import Player from './player.svelte';
	import ShortcutsDialog from './shortcuts-dialog.svelte';
	import { matchShortcut } from '#lib/player/shortcuts.js';
	import {
		positionState,
		setSessionActions,
		setSessionMedia,
		setSessionPlaying,
		setSessionPosition
	} from '#lib/player/media-session.js';
	import type { SleepMode } from '#lib/player/sleep-timer.svelte.js';

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

	async function openShortcuts() {
		if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
		appPlayer.shortcutsOpen = true;
	}

	// Shorts bring their own player, so this one waits out of sight. A party's video plays on
	// in the mini player instead, muted for this tab.
	const onShorts = $derived(page.route.id?.startsWith('/shorts') === true);
	const elsewhere = $derived(!appPlayer.claim && onShorts);
	const silenced = $derived(appPlayer.claim !== null && onShorts);
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

	// A party brings its own queue.
	const queueCount = $derived(appPlayer.claim ? 0 : appPlayer.queue.length);
	let queueOpen = $state(false);
	const queueShown = $derived(mode === 'mini' && queueOpen && queueCount > 0);

	// Phones show a thumbnail-sized video, too small for controls on it.
	const phone = new MediaQuery('width < 40rem');
	const ontoggle = $derived(media.problem ? undefined : () => appPlayer.toggle());
	const onnext = $derived(
		!appPlayer.claim && appPlayer.hasNext ? () => appPlayer.next() : undefined
	);
	const onclose = () => appPlayer.close();

	function ended() {
		appPlayer.paused = true;
		media.events?.onended?.();
		if (appPlayer.claim) return;
		appPlayer.ended = appPlayer.sleep.ended() || !appPlayer.next();
	}

	// Loop and sleep timer would act for everyone in a party, so it has neither.
	function setLoop(on: boolean) {
		appPlayer.loop = on;
		if (on && appPlayer.sleep.mode === 'end') appPlayer.sleep.set('off');
	}

	function setSleep(mode: SleepMode) {
		appPlayer.sleep.set(mode);
		if (mode === 'end') appPlayer.loop = false;
	}

	let now = $state(Date.now());
	$effect(() => {
		if (appPlayer.sleep.deadline === null) return;
		now = Date.now();
		const timer = setInterval(() => (now = Date.now()), 15_000);
		return () => clearInterval(timer);
	});
	const sleepLabel = $derived.by(() => {
		const deadline = appPlayer.sleep.deadline;
		if (deadline === null) return undefined;
		const minutes = Math.max(1, Math.ceil((deadline - now) / 60_000));
		return m.player_sleep_minutes({ minutes });
	});

	// Shift+N, Shift+P and ? on the watch page; the player handles the other keys.
	$effect(() => {
		const onkeydown = (event: KeyboardEvent) => {
			if (mode !== 'inline') return;
			const shortcut = matchShortcut(event);
			if (shortcut === 'help') {
				event.preventDefault();
				void openShortcuts();
			} else if (shortcut === 'next' && appPlayer.hasNext) {
				event.preventDefault();
				appPlayer.next();
			} else if (shortcut === 'previous' && appPlayer.hasPrevious) {
				event.preventDefault();
				appPlayer.previous();
			}
		};
		document.addEventListener('keydown', onkeydown);
		return () => document.removeEventListener('keydown', onkeydown);
	});

	// Lock screen and media keys. In a party, play and pause go through its sync like any other
	// input; members who may not control it are snapped back, so they get no seeking.
	const mediaSession = $derived(mode !== 'hidden' && !silenced && !media.problem);
	$effect(() => {
		if (!mediaSession) return;
		setSessionMedia({ title: media.title, artist: media.channel?.name, artwork: media.poster });
		return () => setSessionMedia(null);
	});
	$effect(() => {
		if (!mediaSession) return;
		const mayControl = appPlayer.claim?.mayControl?.() ?? true;
		setSessionActions({
			play: () => appPlayer.play(),
			pause: () => appPlayer.pause(),
			seekTo: mayControl ? (seconds) => appPlayer.setTime(seconds) : undefined,
			position: () => appPlayer.position(),
			next: appPlayer.hasNext ? () => appPlayer.next() : undefined,
			previous: appPlayer.hasPrevious ? () => appPlayer.previous() : undefined
		});
		return () => setSessionActions(null);
	});
	$effect(() => {
		if (!mediaSession) return;
		setSessionPlaying(!appPlayer.paused);
		return () => setSessionPlaying(null);
	});

	let reported = { at: 0, position: 0, rate: 1, duration: -1 };
	// Play, pause and seeks change what the system extrapolates, so the next update is sent.
	const resetPosition = () => (reported.duration = -1);
	$effect(() => {
		void appPlayer.generation;
		resetPosition();
		return () => setSessionPosition(null);
	});
	/** Position for the system's progress bar, sent when it differs from what it extrapolates. */
	function reportPosition(position: number, duration: number) {
		if (!mediaSession) return;
		const rate = appPlayer.mediaElement()?.playbackRate ?? 1;
		const at = performance.now();
		const moving = appPlayer.paused ? 0 : reported.rate;
		const expected = reported.position + ((at - reported.at) / 1000) * moving;
		if (
			duration === reported.duration &&
			rate === reported.rate &&
			Math.abs(expected - position) < 1
		) {
			return;
		}
		reported = { at, position, rate, duration };
		setSessionPosition(positionState(position, duration, rate));
	}
</script>

<SponsorBlockDialog bind:open={sponsorBlockOpen} />
<ShortcutsDialog bind:open={appPlayer.shortcutsOpen} />

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
>
	<div class="app-player-video">
		{#key appPlayer.generation}
			{#if media.problem}
				<PlayerMessage
					problem={media.problem}
					videoId={media.videoId}
					poster={media.poster}
					onretry={media.onretry}
					compact={mode === 'mini'}
				/>
			{:else}
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
					basicHotkeys={mode === 'mini'}
					loop={appPlayer.loop}
					onloopchange={appPlayer.claim ? undefined : setLoop}
					sleep={appPlayer.sleep.mode}
					{sleepLabel}
					onsleepchange={appPlayer.claim ? undefined : setSleep}
					onshortcuts={openShortcuts}
					autoplay={media.autoplay ?? true}
					{segments}
					externalRate={media.externalRate}
					onsponsorblocksettings={session.sponsorblock ? openSponsorBlock : undefined}
					ontheatertoggle={options.ontheatertoggle}
					onprogress={media.trackProgress === false ? undefined : reportProgress(media.videoId)}
					onplay={(time) => {
						resetPosition();
						appPlayer.paused = false;
						appPlayer.ended = false;
						media.events?.onplay?.(time);
					}}
					onpause={(time) => {
						resetPosition();
						appPlayer.paused = true;
						media.events?.onpause?.(time);
					}}
					onseeked={(time) => {
						resetPosition();
						media.events?.onseeked?.(time);
					}}
					ontimeupdate={(time, duration) => {
						reportPosition(time, duration);
						media.events?.ontimeupdate?.(time, duration);
					}}
					onended={ended}
					{silenced}
				/>
			{/if}
		{/key}
		{@render appPlayer.claim?.overlay?.()}
		{#if mode === 'mini' && phone.current}
			<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by the watch page -->
			<a
				href={media.href}
				class="app-player-expand"
				aria-label={m.mini_player_expand()}
				data-testid="mini-player-expand"
			></a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
		{:else if mode === 'mini'}
			<MiniPlayerOverlay
				href={media.href}
				paused={appPlayer.paused}
				{ontoggle}
				{onnext}
				{onclose}
				closeLabel={appPlayer.claim?.closeLabel?.()}
			/>
		{/if}
	</div>
	{#if mode === 'mini'}
		<MiniPlayerBar
			title={media.title}
			channel={media.channel?.name}
			href={media.href}
			paused={appPlayer.paused}
			muted={silenced}
			ontoggle={phone.current ? ontoggle : undefined}
			onnext={phone.current ? onnext : undefined}
			{queueCount}
			queueOpen={queueShown}
			onqueuetoggle={() => (queueOpen = !queueShown)}
			closeLabel={appPlayer.claim?.closeLabel?.()}
			onclose={phone.current ? onclose : undefined}
			class="app-player-bar"
		/>
		{#if queueShown}
			<UpNext
				class="app-player-queue rounded-none border-0 border-t bg-transparent"
				listClass="max-h-[min(18rem,50svh)]"
			/>
		{/if}
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

	.app-player[data-flush] :global([data-testid='player-message']),
	.app-player[data-mode='mini'] :global([data-testid='player-message']) {
		border-radius: 0;
	}

	.app-player[data-flush] :global(.oxi-player-frame) {
		--media-video-border-radius: 0;
	}

	.app-player[data-flush] :global(.oxi-player-frame)::after {
		border-color: transparent;
	}

	.app-player[data-mode='mini'] {
		position: fixed;
		/* Half a rem inside the body card, which ends before the party sidebar. */
		inset-inline-end: calc(var(--content-inset-end, 0.5rem) + 0.5rem);
		bottom: calc(1rem + env(safe-area-inset-bottom));
		z-index: 40;
		transition: inset-inline-end 200ms linear;
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

	@media (prefers-reduced-motion: reduce) {
		.app-player[data-mode='mini'] {
			transition: none;
		}
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

	/* The mini player is a preview: its own controls replace the player's. */
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

		.app-player[data-mode='mini'] {
			flex-wrap: wrap;
		}

		.app-player[data-mode='mini'] :global(.app-player-queue) {
			flex-basis: 100%;
		}
	}
</style>
