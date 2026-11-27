<script lang="ts" module>
	export type ShortsFeed = 'discover' | 'subscriptions';
</script>

<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import ClapperboardIcon from '@lucide/svelte/icons/clapperboard';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createBlocksQuery } from '#lib/api/blocks.js';
	import { createChannelTabQuery } from '#lib/api/channel.js';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { createShortsDiscoveryQuery } from '#lib/api/shorts.js';
	import { createFeedQuery } from '#lib/api/subscriptions.js';
	import { useVideo } from '#lib/api/video.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
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
		discoveryQueue,
		markShortSeen,
		needsMore,
		seenShorts,
		shortSlots,
		shortStub,
		takeShortsList
	} from '#lib/shorts/queue.js';
	import type { VideoSummary } from '#lib/video.js';
	import ShortSlot from './short-slot.svelte';

	let {
		openedId,
		feed
	}: {
		/** The short opened from outside; it plays first. */
		openedId?: string;
		/** Where the following shorts come from; without one, the opened short's list or channel. */
		feed?: ShortsFeed;
	} = $props();

	// Read once: the viewer is recreated for every short opened from outside and every feed.
	// svelte-ignore state_referenced_locally
	const opened = openedId;
	// svelte-ignore state_referenced_locally
	const source: ShortsFeed | undefined = feed ?? (opened ? undefined : 'discover');
	const context = opened && !source ? takeShortsList(opened) : undefined;
	const seenBefore = source === 'discover' ? seenShorts() : new Set<string>();

	let currentId = $state(opened);
	let sourceChannel = $state<string>();
	/** Finger offset in px while dragging, else 0. */
	let drag = $state(0);
	let dragging = $state(false);

	const feedQuery =
		source === 'discover'
			? createShortsDiscoveryQuery(() => true)
			: source === 'subscriptions'
				? createFeedQuery(() => false, 'shorts')
				: undefined;
	const feedItems = $derived.by(() => {
		const items = feedQuery?.data?.pages.flatMap((p) => p.items) ?? [];
		return source === 'discover'
			? discoveryQueue(items, seenBefore)
			: uniqueBy(items, (short) => short.id);
	});

	// Without a list to continue, the next shorts come from the opened short's channel.
	const openedVideo = useVideo(() => (opened && !source && !context ? opened : null));
	$effect(() => {
		const details = openedVideo.data;
		if (sourceChannel !== undefined || !details || details.id !== opened) return;
		sourceChannel = details.channel.id;
	});
	const channelShorts = createChannelTabQuery(
		() => sourceChannel || undefined,
		() => 'shorts',
		() => 'latest'
	);

	const blocks = createBlocksQuery();
	const blocked = $derived(new Set(blocks.data?.map((c) => c.id)));
	const listed = $derived(
		source
			? opened
				? [
						feedItems.find((short) => short.id === opened) ?? shortStub(opened),
						...feedItems.filter((short) => short.id !== opened)
					]
				: feedItems
			: buildQueue(opened ?? '', context, channelShorts.data?.pages.flatMap((p) => p.items) ?? [])
	);
	const activeId = $derived(currentId ?? listed.find((s) => !blocked.has(s.channel.id))?.id);
	// Shorts of blocked channels drop out, except the one on screen.
	const queue = $derived(
		listed.filter((short) => short.id === activeId || !blocked.has(short.channel.id))
	);
	const index = $derived(queue.findIndex((short) => short.id === activeId));
	const current = $derived(activeId ? (queue[index] ?? shortStub(activeId)) : undefined);
	const previous = $derived(index > 0 ? queue[index - 1] : undefined);
	const next = $derived(index >= 0 ? queue[index + 1] : undefined);
	const slots = $derived(current ? shortSlots(current, previous, next) : []);

	$effect(() => {
		if (source === 'discover' && activeId) markShortSeen(activeId);
	});

	// Pages can come back empty once blocked or seen shorts are left out, so this keeps loading.
	$effect(() => {
		if (activeId ? !needsMore(queue, index) : !feedQuery?.data) return;
		const more = feedQuery ?? channelShorts;
		if (more.hasNextPage && !more.isFetching) void more.fetchNextPage();
	});

	const video = useVideo(() => activeId ?? null);
	const shown = $derived(video.data?.id === activeId ? video.data : undefined);
	const title = $derived(shown?.title ?? current?.title);
	const feedTitle = $derived(
		source === 'subscriptions' ? m.shorts_feed_subscriptions() : m.nav_shorts()
	);
	const emptyFeed = $derived(
		!activeId && !!feedQuery?.isSuccess && !feedQuery.hasNextPage && !feedQuery.isFetching
	);
	const feeds = [
		{ kind: 'discover', label: m.shorts_feed_discover, href: resolve('/shorts') },
		{
			kind: 'subscriptions',
			label: m.shorts_feed_subscriptions,
			href: `${resolve('/shorts')}?feed=subscriptions`
		}
	] as const;

	const canStep = (step: Step) => (step === 1 ? next : previous) !== undefined;

	function show(target: VideoSummary | undefined) {
		if (!target) return;
		currentId = target.id;
	}

	// Replaces the entry, so going back leaves the shorts instead of stepping through them. The feed
	// stays in the URL, so a reload continues it from the same short.
	$effect(() => {
		if (!activeId) return;
		const search = source ? `?feed=${source}` : '';
		const path = resolve('/shorts/[id=videoid]', { id: activeId });
		if (location.pathname + location.search === path + search) return;
		// eslint-disable-next-line svelte/no-navigation-without-resolve -- path resolved above
		void replaceState(`${path}${search}`, {});
	});

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
	<title>{pageTitle(title || (source ? feedTitle : m.channel_tab_shorts()))}</title>
</svelte:head>

<svelte:window {onkeydown} />

<section class="shorts" aria-label={m.shorts_player_label()} data-testid="shorts-viewer">
	{#if source && session.user}
		<nav class="shorts-feeds" aria-label={m.shorts_feed_label()} data-testid="shorts-feeds">
			{#each feeds as item (item.kind)}
				<a
					href={item.href}
					data-sveltekit-replacestate
					aria-current={source === item.kind ? 'page' : undefined}
				>
					{item.label()}
				</a>
			{/each}
		</nav>
	{/if}
	<div class="shorts-row">
		<div class="shorts-stage" data-dragging={dragging ? '' : undefined} {@attach gestures}>
			{#if !current}
				<div class="shorts-placeholder" data-testid="shorts-feed-state">
					{#if feedQuery?.isError}
						<ErrorState onretry={() => void feedQuery.refetch()} class="m-4" />
					{:else if emptyFeed}
						<EmptyState
							icon={ClapperboardIcon}
							title={source === 'subscriptions'
								? m.shorts_subscriptions_empty_title()
								: m.shorts_discover_empty_title()}
							description={source === 'subscriptions'
								? m.shorts_subscriptions_empty_body()
								: m.shorts_discover_empty_body()}
							class="m-4 border-white/20 text-white [&_p]:text-white/70"
						/>
					{:else}
						<Skeleton
							class="size-full rounded-none bg-white/10"
							role="status"
							aria-label={m.shorts_loading()}
						/>
					{/if}
				</div>
			{/if}
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
	</div>
</section>

<style>
	.shorts {
		--shorts-height: calc(100svh - 3.5rem);
		--shorts-gap: 0px;
		display: flex;
		flex-direction: column;
		height: var(--shorts-height);
		overflow: hidden;
		background: oklch(0.13 0.004 286);
	}

	.shorts-row {
		display: flex;
		flex: 1;
		min-height: 0;
		justify-content: center;
		align-items: center;
		gap: 1rem;
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

	.shorts-placeholder {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		background: oklch(0.13 0.004 286);
		color: white;
	}

	/* Its own row above the shorts, so it never covers the video. */
	.shorts-feeds {
		align-self: center;
		flex-shrink: 0;
		margin-block: 0.5rem;
		display: flex;
		gap: 0.25rem;
		padding: 0.25rem;
		border-radius: 999px;
		background: rgb(255 255 255 / 8%);
	}

	.shorts-feeds a {
		padding: 0.25rem 0.75rem;
		border-radius: 999px;
		font-size: 0.8125rem;
		font-weight: 500;
		line-height: 1.25rem;
		white-space: nowrap;
		color: rgb(255 255 255 / 75%);
		transition:
			background-color 150ms,
			color 150ms;
	}

	.shorts-feeds a:hover {
		color: white;
	}

	.shorts-feeds a[aria-current='page'] {
		background: white;
		color: oklch(0.21 0.006 286);
	}

	.shorts-feeds a:focus-visible {
		outline: 2px solid var(--color-ring, white);
		outline-offset: 2px;
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

		.shorts-placeholder {
			border-radius: var(--radius-xl, 0.75rem);
		}

		/* On the page surface, so it follows the theme instead of the dark mobile stage. */
		.shorts-feeds {
			margin-block: 0 0.75rem;
			background: var(--color-muted);
		}

		.shorts-feeds a {
			color: var(--color-muted-foreground);
		}

		.shorts-feeds a:hover {
			color: var(--color-foreground);
		}

		.shorts-feeds a[aria-current='page'] {
			background: var(--color-background);
			color: var(--color-foreground);
			box-shadow: 0 1px 2px rgb(0 0 0 / 8%);
		}
	}
</style>
