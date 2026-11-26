<script lang="ts">
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { page } from '$app/state';
	import { manifestUrl, pickThumbnail, useManifest, useVideo } from '#lib/api/video.js';
	import Player, { type PlayerSubtitle } from '#lib/components/player/player.svelte';
	import type { ProgressReport } from '#lib/components/player/progress.js';
	import { parseStartTime } from '#lib/components/player/start-time.js';
	import { readJson, writeJson } from '#lib/components/player/storage.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { describeProblem, liveProblem } from '#lib/components/watch/errors.js';
	import {
		readLocalProgress,
		resumePosition,
		saveLocalProgress
	} from '#lib/components/watch/local-progress.js';
	import PlayerMessage from '#lib/components/watch/player-message.svelte';
	import { playableRelated } from '#lib/components/watch/related.js';
	import RelatedVideos from '#lib/components/watch/related-videos.svelte';
	import VideoDetailsSkeleton from '#lib/components/watch/video-details-skeleton.svelte';
	import VideoDetails from '#lib/components/watch/video-details.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';

	const THEATER_KEY = 'oxistream:theater';

	const videoId = $derived(page.url.searchParams.get('v'));
	const timeParam = $derived(page.url.searchParams.get('t'));

	const video = useVideo(() => videoId);
	const manifest = useManifest(() => videoId);

	let theater = $state(readJson(THEATER_KEY, (v): v is boolean => typeof v === 'boolean') ?? false);
	let player = $state<ReturnType<typeof Player>>();
	let descriptionExpanded = $state(false);

	const details = $derived(video.data);
	const problem = $derived.by(() => {
		if (video.isError) return describeProblem(video.error);
		if (details?.is_live) return liveProblem();
		if (manifest.isError) return describeProblem(manifest.error);
		return null;
	});
	const poster = $derived(details ? pickThumbnail(details.thumbnails, 1280) : undefined);
	const subtitles = $derived<PlayerSubtitle[]>(
		(details?.subtitles ?? []).map((s) => ({
			id: s.id,
			lang: s.lang,
			url: s.url,
			label: s.auto_generated ? m.player_auto_generated({ label: s.label }) : s.label
		}))
	);
	const related = $derived(details ? playableRelated(details.related) : []);
	const title = $derived(
		details
			? pageTitle(details.title)
			: video.isError
				? pageTitle(problem?.title)
				: pageTitle(m.watch_title())
	);

	// Read once per video: an explicit ?t= wins over where this browser left off.
	const startTime = $derived.by(() => {
		if (!videoId) return 0;
		return parseStartTime(timeParam) ?? resumePosition(readLocalProgress(videoId)) ?? 0;
	});

	// A new ?t= for the video that is already playing, e.g. from a link in its description.
	let lastTime: string | null = null;
	$effect(() => {
		const t = timeParam;
		if (lastTime !== null && t !== lastTime) {
			const seconds = parseStartTime(t);
			if (seconds !== null) player?.seek(seconds);
		}
		lastTime = t ?? '';
	});

	$effect(() => {
		void videoId;
		descriptionExpanded = false;
	});

	function toggleTheater() {
		theater = !theater;
		writeJson(THEATER_KEY, theater);
	}

	function saveProgress(id: string) {
		// Anonymous fallback. Signed-in users will report to the server (#18).
		return (report: ProgressReport) => saveLocalProgress(id, report);
	}

	function seek(seconds: number) {
		player?.seek(seconds);
		document.getElementById('player')?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
	}

	function retry() {
		if (video.isError) void video.refetch();
		else void manifest.refetch();
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 't' || event.ctrlKey || event.metaKey || event.altKey) return;
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"]'))
			return;
		if (!window.matchMedia('(width >= 64rem)').matches) return;
		event.preventDefault();
		toggleTheater();
	}
</script>

<svelte:head>
	<title>{title}</title>
</svelte:head>

<svelte:window {onkeydown} />

{#if !videoId}
	<div class="px-(--page-gutter) py-6">
		<EmptyState
			icon={TvMinimalPlayIcon}
			title={m.watch_no_video_title()}
			description={m.watch_no_video_body()}
			headingLevel={2}
		/>
	</div>
{:else}
	<div class="@container flex-1">
		<div class="watch" data-theater={theater ? '' : undefined}>
			<div id="player" class="watch-player">
				{#if problem}
					<PlayerMessage
						{problem}
						{videoId}
						{poster}
						onretry={problem.retry ? retry : undefined}
						class="watch-frame"
					/>
				{:else if details}
					{#key videoId}
						<Player
							bind:this={player}
							src={manifestUrl(videoId)}
							{poster}
							{subtitles}
							{startTime}
							{theater}
							autoplay
							ontheatertoggle={toggleTheater}
							onprogress={saveProgress(videoId)}
							class="watch-frame"
						/>
					{/key}
				{:else}
					<div class="watch-frame" role="status" aria-label={m.watch_loading()}>
						<Skeleton class="aspect-video w-full rounded-xl" />
					</div>
				{/if}
			</div>

			<div class="watch-meta">
				{#if details}
					<VideoDetails video={details} bind:descriptionExpanded onseek={seek} />
				{:else if video.isPending}
					<VideoDetailsSkeleton />
				{/if}
			</div>

			{#if details || video.isPending}
				<RelatedVideos videos={related} loading={video.isPending} class="watch-related" />
			{/if}
		</div>
	</div>
{/if}

<style>
	.watch {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-template-areas: 'player' 'meta' 'related';
		gap: 1rem 1.5rem;
		width: 100%;
		max-width: 112rem;
		margin-inline: auto;
		padding: 1rem var(--page-gutter) 2.5rem;
	}

	.watch-player {
		grid-area: player;
		scroll-margin-top: 4.5rem;
	}

	.watch-meta {
		grid-area: meta;
		min-width: 0;
	}

	.watch :global(.watch-related) {
		grid-area: related;
		min-width: 0;
		margin-top: 0.5rem;
	}

	@container (width >= 60rem) {
		.watch {
			grid-template-columns: minmax(0, 1fr) 22rem;
			grid-template-rows: auto 1fr;
			grid-template-areas: 'player related' 'meta related';
		}

		.watch :global(.watch-related) {
			margin-top: 0;
		}

		.watch[data-theater] {
			grid-template-areas: 'player player' 'meta related';
		}
	}

	@container (width >= 48rem) {
		/* Theater mode: a dark band across the whole content area, the video as large as fits. */
		.watch[data-theater] {
			padding-top: 0;
		}

		.watch[data-theater] .watch-player {
			margin-inline: calc(var(--page-gutter) * -1);
			margin-bottom: 0.5rem;
			background: oklch(0.13 0.004 286);
		}

		.watch[data-theater] :global(.watch-frame) {
			width: min(100%, calc((100svh - 9rem) * 16 / 9));
			margin-inline: auto;
		}

		.watch[data-theater] :global(.watch-frame [data-slot='skeleton']),
		.watch[data-theater] :global(.watch-frame[data-testid='player-message']) {
			border-radius: 0;
		}
	}
</style>
