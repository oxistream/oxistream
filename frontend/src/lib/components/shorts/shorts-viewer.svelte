<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { cubicOut } from 'svelte/easing';
	import { fly } from 'svelte/transition';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createChannelTabQuery } from '#lib/api/channel.js';
	import { createProgressReporter } from '#lib/api/history.js';
	import { pickImage, toChannel } from '#lib/api/mapping.js';
	import { manifestUrl, prefetchVideo, useManifest, useVideo } from '#lib/api/video.js';
	import Player from '#lib/components/player/player.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { describeProblem, liveProblem } from '#lib/components/watch/errors.js';
	import PlayerMessage from '#lib/components/watch/player-message.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { createWheelPager, keyStep, swipeStep, type Step } from '#lib/shorts/gestures.js';
	import { createLoopingReporter } from '#lib/shorts/progress.js';
	import { buildQueue, needsMore, shortStub, takeShortsList } from '#lib/shorts/queue.js';
	import type { ChannelSummary } from '#lib/video.js';
	import ShortOverlay from './short-overlay.svelte';

	let { openedId }: { openedId: string } = $props();

	const client = useQueryClient();
	// Read once: the viewer is recreated for every short opened from outside.
	// svelte-ignore state_referenced_locally
	const context = takeShortsList(openedId);
	// svelte-ignore state_referenced_locally
	let currentId = $state(openedId);
	let direction = $state<Step>(1);
	let sourceChannel = $state<string>();

	const video = useVideo(() => currentId);
	const manifest = useManifest(() => currentId);
	const details = $derived(video.data);

	// Without a list to continue, the next shorts come from the opened short's channel.
	$effect(() => {
		if (context || sourceChannel !== undefined || !details || details.id !== openedId) return;
		sourceChannel = details.channel.id;
	});
	const channelShorts = createChannelTabQuery(
		() => sourceChannel || undefined,
		() => 'shorts',
		() => 'latest'
	);

	const queue = $derived(
		buildQueue(openedId, context, channelShorts.data?.pages.flatMap((p) => p.items) ?? [])
	);
	const index = $derived(queue.findIndex((short) => short.id === currentId));
	const current = $derived(queue[index] ?? shortStub(currentId));
	const previous = $derived(index > 0 ? queue[index - 1] : undefined);
	const next = $derived(index >= 0 ? queue[index + 1] : undefined);

	$effect(() => {
		if (!needsMore(queue, index)) return;
		if (channelShorts.hasNextPage && !channelShorts.isFetching) void channelShorts.fetchNextPage();
	});

	// Only metadata: the details and the poster, not the stream.
	$effect(() => {
		if (!next) return;
		void prefetchVideo(client, next.id);
		if (next.thumbnailUrl) new Image().src = next.thumbnailUrl;
	});

	const shown = $derived(details?.id === currentId ? details : undefined);
	const title = $derived(shown?.title ?? current.title);
	const channel = $derived<ChannelSummary | undefined>(
		shown ? toChannel(shown.channel) : current.channel.name ? current.channel : undefined
	);
	const poster = $derived(current.thumbnailUrl ?? pickImage(shown?.thumbnails, 720));
	const problem = $derived.by(() => {
		if (video.isError) return describeProblem(video.error);
		if (shown?.is_live) return liveProblem();
		if (manifest.isError) return describeProblem(manifest.error);
		return null;
	});

	const reportProgress = createProgressReporter();
	const progress = $derived(createLoopingReporter(reportProgress(currentId)));

	function go(step: Step) {
		const target = step === 1 ? next : previous;
		if (!target) return;
		direction = step;
		currentId = target.id;
		// Replaces the entry, so going back leaves the shorts instead of stepping through them.
		void replaceState(resolve('/shorts/[id=videoid]', { id: target.id }), {});
	}

	function retry() {
		if (video.isError) void video.refetch();
		else void manifest.refetch();
	}

	function onkeydown(event: KeyboardEvent) {
		const step = keyStep(event);
		if (step === null) return;
		event.preventDefault();
		go(step);
	}

	const CONTROLS = 'media-time-slider, media-volume-slider, media-menu, media-volume-popover';

	function gestures(node: HTMLElement) {
		const wheel = createWheelPager(go);
		let start: { x: number; y: number } | null = null;
		const onWheel = (event: WheelEvent) => {
			if ((event.target as Element).closest(CONTROLS)) return;
			event.preventDefault();
			wheel(event);
		};
		const onTouchStart = (event: TouchEvent) => {
			const touch = event.touches[0];
			start =
				event.touches.length === 1 && !(event.target as Element).closest(CONTROLS)
					? { x: touch.clientX, y: touch.clientY }
					: null;
		};
		const onTouchEnd = (event: TouchEvent) => {
			if (!start) return;
			const touch = event.changedTouches[0];
			const step = swipeStep(start, { x: touch.clientX, y: touch.clientY });
			start = null;
			if (step) go(step);
		};
		node.addEventListener('wheel', onWheel, { passive: false });
		node.addEventListener('touchstart', onTouchStart, { passive: true });
		node.addEventListener('touchend', onTouchEnd, { passive: true });
		return () => {
			node.removeEventListener('wheel', onWheel);
			node.removeEventListener('touchstart', onTouchStart);
			node.removeEventListener('touchend', onTouchEnd);
		};
	}

	const reducedMotion =
		typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
</script>

<svelte:head>
	<title>{pageTitle(title || m.channel_tab_shorts())}</title>
</svelte:head>

<svelte:window {onkeydown} />

<section class="shorts" aria-label={m.shorts_player_label()} data-testid="shorts-viewer">
	<div class="shorts-stage" {@attach gestures}>
		{#key currentId}
			<div
				class="shorts-frame"
				data-short-id={currentId}
				in:fly={{ y: direction * 48, duration: reducedMotion ? 0 : 220, easing: cubicOut }}
			>
				{#if problem}
					<PlayerMessage
						{problem}
						videoId={currentId}
						{poster}
						onretry={problem.retry ? retry : undefined}
						class="aspect-auto h-full"
					/>
				{:else if shown}
					<Player
						src={manifestUrl(currentId)}
						{poster}
						variant="short"
						loop
						autoplay
						ontimeupdate={progress.update}
						onprogress={progress.report}
						class="shorts-player"
					/>
				{:else}
					<div class="relative size-full" role="status" aria-label={m.shorts_loading()}>
						{#if poster}
							<img src={poster} alt="" class="size-full object-cover opacity-60" />
						{:else}
							<Skeleton class="size-full rounded-none" />
						{/if}
					</div>
				{/if}
				<ShortOverlay videoId={currentId} {title} {channel} />
			</div>
		{/key}
	</div>

	<div class="shorts-nav">
		<Button
			variant="secondary"
			size="icon-lg"
			class="rounded-full"
			disabled={!previous}
			aria-label={m.shorts_previous()}
			title={m.shorts_previous()}
			aria-keyshortcuts="ArrowUp k"
			onclick={() => go(-1)}
			data-testid="shorts-previous"
		>
			<ChevronUpIcon />
		</Button>
		<Button
			variant="secondary"
			size="icon-lg"
			class="rounded-full"
			disabled={!next}
			aria-label={m.shorts_next()}
			title={m.shorts_next()}
			aria-keyshortcuts="ArrowDown j"
			onclick={() => go(1)}
			data-testid="shorts-next"
		>
			<ChevronDownIcon />
		</Button>
	</div>
</section>

<style>
	.shorts {
		--shorts-height: calc(100svh - 3.5rem);
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		height: var(--shorts-height);
		overflow: hidden;
		background: oklch(0.13 0.004 286);
	}

	.shorts-stage {
		position: relative;
		height: 100%;
		width: 100%;
		touch-action: pinch-zoom;
		overscroll-behavior: contain;
	}

	.shorts-frame {
		position: relative;
		height: 100%;
		width: 100%;
		overflow: hidden;
		background: oklch(0.13 0.004 286);
	}

	.shorts-frame :global(.shorts-player),
	.shorts-frame :global(.shorts-player .oxi-player-frame) {
		height: 100%;
		width: 100%;
	}

	.shorts-frame :global(.shorts-player .oxi-player-frame) {
		aspect-ratio: auto;
		--media-video-border-radius: 0;
	}

	.shorts-frame :global(.shorts-player .oxi-player-frame)::after {
		border-color: transparent;
	}

	.shorts-frame :global([data-testid='player-message']) {
		border-radius: 0;
	}

	.shorts-nav {
		display: none;
	}

	/* Beside the inset sidebar: a portrait card with the buttons next to it, on the page surface. */
	@media (width >= 48rem) {
		.shorts {
			--shorts-height: calc(100svh - 4.5rem);
			padding-block: 1rem;
			background: none;
		}

		.shorts-stage {
			width: auto;
			aspect-ratio: 9 / 16;
			max-width: calc(100% - 4rem);
		}

		.shorts-frame {
			border-radius: var(--radius-xl, 0.75rem);
		}

		.shorts-nav {
			display: flex;
			flex-direction: column;
			gap: 0.75rem;
		}
	}
</style>
