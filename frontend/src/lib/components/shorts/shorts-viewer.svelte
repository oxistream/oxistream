<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createBlocksQuery } from '#lib/api/blocks.js';
	import { createChannelTabQuery } from '#lib/api/channel.js';
	import { useVideo } from '#lib/api/video.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		createWheelPager,
		dragOffset,
		isVerticalDrag,
		keyStep,
		releaseStep,
		type Step
	} from '#lib/shorts/gestures.js';
	import {
		buildQueue,
		needsMore,
		shortSlots,
		shortStub,
		takeShortsList
	} from '#lib/shorts/queue.js';
	import type { VideoSummary } from '#lib/video.js';
	import ShortSlot from './short-slot.svelte';

	let { openedId }: { openedId: string } = $props();

	// Read once: the viewer is recreated for every short opened from outside.
	// svelte-ignore state_referenced_locally
	const context = takeShortsList(openedId);
	// svelte-ignore state_referenced_locally
	let currentId = $state(openedId);
	let sourceChannel = $state<string>();
	/** Finger offset in px while dragging, else 0. */
	let drag = $state(0);
	let dragging = $state(false);

	const video = useVideo(() => currentId);
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

	const blocks = createBlocksQuery();
	const blocked = $derived(new Set(blocks.data?.map((c) => c.id)));
	// Shorts of blocked channels drop out, except the one on screen.
	const queue = $derived(
		buildQueue(openedId, context, channelShorts.data?.pages.flatMap((p) => p.items) ?? []).filter(
			(short) => short.id === currentId || !blocked.has(short.channel.id)
		)
	);
	const index = $derived(queue.findIndex((short) => short.id === currentId));
	const current = $derived(queue[index] ?? shortStub(currentId));
	const previous = $derived(index > 0 ? queue[index - 1] : undefined);
	const next = $derived(index >= 0 ? queue[index + 1] : undefined);
	const slots = $derived(shortSlots(current, previous, next));

	$effect(() => {
		if (!needsMore(queue, index)) return;
		if (channelShorts.hasNextPage && !channelShorts.isFetching) void channelShorts.fetchNextPage();
	});

	const shown = $derived(details?.id === currentId ? details : undefined);
	const title = $derived(shown?.title ?? current.title);

	const canStep = (step: Step) => (step === 1 ? next : previous) !== undefined;

	function show(target: VideoSummary | undefined) {
		if (!target) return;
		currentId = target.id;
		// Replaces the entry, so going back leaves the shorts instead of stepping through them.
		void replaceState(resolve('/shorts/[id=videoid]', { id: target.id }), {});
	}

	function go(step: Step) {
		show(step === 1 ? next : previous);
	}

	/** Moves on to the next short of another channel, if there is one. */
	function skipChannel(channelId: string) {
		show(queue.slice(index + 1).find((short) => short.channel.id !== channelId));
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
		let touch: {
			id: number;
			x: number;
			y: number;
			lastY: number;
			lastTime: number;
			velocity: number;
		} | null = null;

		const onWheel = (event: WheelEvent) => {
			if ((event.target as Element).closest(CONTROLS)) return;
			event.preventDefault();
			wheel(event);
		};
		const onPointerDown = (event: PointerEvent) => {
			if (event.pointerType === 'mouse' || !event.isPrimary) return;
			if ((event.target as Element).closest(CONTROLS)) return;
			touch = {
				id: event.pointerId,
				x: event.clientX,
				y: event.clientY,
				lastY: event.clientY,
				lastTime: event.timeStamp,
				velocity: 0
			};
		};
		const onPointerMove = (event: PointerEvent) => {
			if (!touch || event.pointerId !== touch.id) return;
			const dy = event.clientY - touch.y;
			if (!dragging) {
				if (!isVerticalDrag(event.clientX - touch.x, dy)) return;
				dragging = true;
				node.setPointerCapture(event.pointerId);
			}
			const elapsed = event.timeStamp - touch.lastTime;
			if (elapsed > 0) touch.velocity = (event.clientY - touch.lastY) / elapsed;
			touch.lastY = event.clientY;
			touch.lastTime = event.timeStamp;
			drag = dragOffset(dy, canStep);
		};
		const onPointerEnd = (event: PointerEvent) => {
			if (!touch || event.pointerId !== touch.id) return;
			const wasDragging = dragging;
			const dy = event.clientY - touch.y;
			// A finger resting before release is no flick.
			const velocity = event.timeStamp - touch.lastTime > 100 ? 0 : touch.velocity;
			touch = null;
			if (!wasDragging) return;
			// Keeps the player from reading the end of a drag as a tap.
			event.stopPropagation();
			dragging = false;
			drag = 0;
			const step = event.type === 'pointerup' ? releaseStep(dy, velocity, node.clientHeight) : null;
			if (step) go(step);
		};

		node.addEventListener('wheel', onWheel, { passive: false });
		node.addEventListener('pointerdown', onPointerDown);
		node.addEventListener('pointermove', onPointerMove);
		node.addEventListener('pointerup', onPointerEnd, { capture: true });
		node.addEventListener('pointercancel', onPointerEnd, { capture: true });
		return () => {
			node.removeEventListener('wheel', onWheel);
			node.removeEventListener('pointerdown', onPointerDown);
			node.removeEventListener('pointermove', onPointerMove);
			node.removeEventListener('pointerup', onPointerEnd, { capture: true });
			node.removeEventListener('pointercancel', onPointerEnd, { capture: true });
		};
	}
</script>

<svelte:head>
	<title>{pageTitle(title || m.channel_tab_shorts())}</title>
</svelte:head>

<svelte:window {onkeydown} />

<section class="shorts" aria-label={m.shorts_player_label()} data-testid="shorts-viewer">
	<div class="shorts-stage" data-dragging={dragging ? '' : undefined} {@attach gestures}>
		{#each slots as slot (slot.short.id)}
			<div
				class="shorts-cell"
				style:translate="0 calc({slot.offset} * (100% + var(--shorts-gap)) + {drag}px)"
			>
				<ShortSlot
					short={slot.short}
					active={slot.offset === 0}
					preload={slot.preload}
					onblocked={skipChannel}
				/>
			</div>
		{/each}
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
		--shorts-gap: 0px;
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
		overflow: hidden;
		touch-action: pinch-zoom;
		overscroll-behavior: contain;
	}

	/* A vertical carousel: neighbours sit directly above and below, moved by translate only. */
	.shorts-cell {
		position: absolute;
		inset: 0;
		transition: translate 280ms cubic-bezier(0.25, 1, 0.5, 1);
		will-change: translate;
	}

	.shorts-stage[data-dragging] .shorts-cell {
		transition: none;
	}

	@media (prefers-reduced-motion: reduce) {
		.shorts-cell {
			transition: none;
		}
	}

	.shorts-nav {
		display: none;
	}

	/* Beside the inset sidebar: a portrait card with the buttons next to it, on the page surface. */
	@media (width >= 48rem) {
		.shorts {
			--shorts-height: calc(100svh - 4.5rem);
			--shorts-gap: 1rem;
			padding-block: 1rem;
			background: none;
		}

		.shorts-stage {
			width: auto;
			aspect-ratio: 9 / 16;
			max-width: calc(100% - 4rem);
		}

		.shorts-nav {
			display: flex;
			flex-direction: column;
			gap: 0.75rem;
		}
	}
</style>
