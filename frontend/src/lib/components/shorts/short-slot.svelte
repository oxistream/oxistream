<script lang="ts">
	import { pickImage, toChannel } from '#lib/api/mapping.js';
	import { useSegments, useSponsorBlockSettings } from '#lib/api/sponsorblock.svelte.js';
	import { createShortDetailsQuery } from '#lib/api/shorts.js';
	import { manifestUrl, useManifest, useVideo } from '#lib/api/video.js';
	import { createSettingsQuery, playbackPreferences } from '#lib/api/settings.js';
	import Player from '#lib/components/player/player.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { describeProblem, liveProblem } from '#lib/components/watch/errors.js';
	import PlayerMessage from '#lib/components/watch/player-message.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { resolveSegments } from '#lib/sponsorblock/skip.js';
	import type { ChannelSummary, VideoSummary } from '#lib/video.js';
	import ShortOverlay from './short-overlay.svelte';

	let {
		short,
		active,
		preload,
		onblocked
	}: {
		short: VideoSummary;
		/** The visible short plays; the others stay paused and only keep what they loaded. */
		active: boolean;
		/** Load the stream while not visible yet. A player, once created, is kept either way. */
		preload: boolean;
		/** The user blocked the short's channel from its overlay. */
		onblocked?: (channelId: string) => void;
	} = $props();

	// The slot is keyed by the short's id, so it never changes.
	// svelte-ignore state_referenced_locally
	const id = short.id;
	let loaded = $state(false);
	$effect(() => {
		if (active || preload) loaded = true;
	});

	const video = useVideo(() => id);
	const manifest = useManifest(() => (loaded ? id : null));
	const overlay = createShortDetailsQuery(() => (loaded ? id : null));
	const segmentsQuery = useSegments(() => (loaded ? id : null));
	const sponsorBlock = useSponsorBlockSettings();
	const segments = $derived(
		sponsorBlock.ready && segmentsQuery.data
			? resolveSegments(segmentsQuery.data, sponsorBlock.current)
			: []
	);

	const details = $derived(video.data?.id === id ? video.data : undefined);
	const title = $derived(details?.title ?? short.title);
	const channel = $derived<ChannelSummary | undefined>(
		details ? toChannel(details.channel) : short.channel.name ? short.channel : undefined
	);
	const poster = $derived(short.thumbnailUrl ?? pickImage(details?.thumbnails, 720));
	const problem = $derived.by(() => {
		if (video.isError) return describeProblem(video.error);
		if (details?.is_live) return liveProblem();
		if (manifest.isError) return describeProblem(manifest.error);
		return null;
	});

	let player = $state<Player>();
	const settings = createSettingsQuery();

	$effect(() => {
		if (!player) return;
		if (active) player.play();
		else player.pause();
	});

	function retry() {
		if (video.isError) void video.refetch();
		else void manifest.refetch();
	}
</script>

<div
	class="shorts-frame"
	data-short-id={id}
	data-active={active ? '' : undefined}
	aria-hidden={active ? undefined : 'true'}
	inert={!active}
>
	{#if problem}
		<PlayerMessage
			{problem}
			videoId={id}
			{poster}
			onretry={problem.retry ? retry : undefined}
			class="aspect-auto h-full"
		/>
	{:else if details && loaded}
		<Player
			bind:this={player}
			src={manifestUrl(id)}
			{poster}
			variant="short"
			{segments}
			playback={playbackPreferences(settings.data)}
			loop
			autoplay={active}
			globalHotkeys={active}
			class="shorts-player"
		/>
	{:else}
		<div class="relative size-full" role="status" aria-label={m.shorts_loading()}>
			{#if poster}
				<img src={poster} alt="" class="size-full object-cover" />
			{:else}
				<Skeleton class="size-full rounded-none" />
			{/if}
		</div>
	{/if}
	<ShortOverlay
		videoId={id}
		{title}
		{channel}
		linkedVideo={overlay.data?.linked_video ?? undefined}
		disclosures={overlay.data?.content_disclosures}
		onblocked={() => channel?.id && onblocked?.(channel.id)}
	/>
</div>

<style>
	.shorts-frame {
		--shorts-radius: 0px;
		position: relative;
		height: 100%;
		width: 100%;
		overflow: hidden;
		/* clip-path also clips the composited video and poster layers, which a radius alone misses. */
		clip-path: inset(0 round var(--shorts-radius));
		border-radius: var(--shorts-radius);
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

	/* A neighbour sliding in shows its first frame and the overlay, not its paused controls. */
	.shorts-frame:not([data-active]) :global(.video-controls-content),
	.shorts-frame:not([data-active]) :global(.video-controls-backdrop),
	.shorts-frame:not([data-active]) :global(.video-controls-center),
	.shorts-frame:not([data-active]) :global(.media-buffering-indicator) {
		visibility: hidden;
	}

	/* Lets the overlay's gradient sit between the video and the player controls. */
	.shorts-frame :global(.shorts-player .media-container) {
		isolation: auto;
	}

	/* The overlay gradient already backs the controls. */
	.shorts-frame :global(.shorts-player .video-controls-backdrop) {
		display: none;
	}

	.shorts-frame :global(.short-overlay) {
		transition: padding-bottom 150ms cubic-bezier(0, 0, 0.2, 1);
	}

	/* The player controls only show while paused; the overlay content then moves up out of their way
	   while its gradient grows upwards and stays behind the controls. */
	.shorts-frame[data-active]:has(:global(.oxi-player-frame[data-paused])) :global(.short-overlay) {
		padding-bottom: 4.25rem;
	}

	.shorts-frame :global([data-testid='player-message']) {
		border-radius: 0;
	}

	@media (width >= 48rem) {
		.shorts-frame {
			--shorts-radius: var(--radius-xl, 0.75rem);
		}
	}
</style>
