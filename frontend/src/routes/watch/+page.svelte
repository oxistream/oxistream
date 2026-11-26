<script lang="ts">
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ApiError, pickImage } from '#lib/api/mapping.js';
	import { createProgressReporter, createVideoProgressQuery } from '#lib/api/history.js';
	import {
		isLocalPlaylistId,
		isMixId,
		needsMoreItems,
		neighbors
	} from '#lib/api/playlist-order.js';
	import { createPlaylistQuery, useLastPlayed } from '#lib/api/playlists.js';
	import { manifestUrl, useManifest, useVideo } from '#lib/api/video.js';
	import { createYouTubePlaylistQuery, mergeYouTubePlaylist } from '#lib/api/youtube-playlists.js';
	import Player, { type PlayerSubtitle } from '#lib/components/player/player.svelte';
	import { playlistWatchHref } from '#lib/components/playlist/links.js';
	import { localPlayback, youtubePlayback } from '#lib/components/playlist/playback.js';
	import PlaylistPanel from '#lib/components/playlist/playlist-panel.svelte';
	import SaveToPlaylistButton from '#lib/components/playlist/save-to-playlist-button.svelte';
	import { parseStartTime } from '#lib/components/player/start-time.js';
	import { readJson, writeJson } from '#lib/components/player/storage.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { describeProblem, liveProblem } from '#lib/components/watch/errors.js';
	import { readLocalProgress, resumePosition } from '#lib/components/watch/local-progress.js';
	import PlayerMessage from '#lib/components/watch/player-message.svelte';
	import { playableRelated } from '#lib/components/watch/related.js';
	import RelatedVideos from '#lib/components/watch/related-videos.svelte';
	import VideoDetailsSkeleton from '#lib/components/watch/video-details-skeleton.svelte';
	import VideoDetails from '#lib/components/watch/video-details.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const THEATER_KEY = 'oxistream:theater';
	// Enough to find a video anywhere in YouTube's largest playlists (5,000 videos).
	const MAX_PLAYLIST_PAGES = 60;

	const videoId = $derived(page.url.searchParams.get('v'));
	const timeParam = $derived(page.url.searchParams.get('t'));

	const listId = $derived(page.url.searchParams.get('list'));

	const video = useVideo(() => videoId);
	const playlist = createPlaylistQuery(() => listId);
	const youtubePlaylist = createYouTubePlaylistQuery(() => listId);
	const rememberLastPlayed = useLastPlayed();
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
	const poster = $derived(details ? pickImage(details.thumbnails, 1280) : undefined);
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

	const localList = $derived(isLocalPlaylistId(listId));
	const youtubeList = $derived(mergeYouTubePlaylist(youtubePlaylist.data));
	const playback = $derived.by(() => {
		if (localList) return playlist.data ? localPlayback(playlist.data) : undefined;
		return youtubeList ? youtubePlayback(youtubeList, youtubePlaylist.hasNextPage) : undefined;
	});
	const playlistPosition = $derived(playback ? neighbors(playback.items, videoId) : { index: -1 });
	const playlistStatus = $derived.by(() => {
		if (!listId) return null;
		if (isMixId(listId)) return 'mix';
		if (playback) return 'ready';
		const error = localList ? playlist.error : youtubePlaylist.error;
		if (!error) return 'loading';
		return error instanceof ApiError && error.reason === 'mix' ? 'mix' : 'unavailable';
	});

	// YouTube playlists come in pages: load until the current video and the one after it are known.
	$effect(() => {
		const query = youtubePlaylist;
		if (!youtubeList || !query.hasNextPage || query.isFetching || query.isFetchNextPageError)
			return;
		if ((query.data?.pages.length ?? 0) >= MAX_PLAYLIST_PAGES) return;
		if (needsMoreItems(youtubeList.items, videoId)) void query.fetchNextPage();
	});

	// The owner's resume point for "Continue" on the playlist page.
	$effect(() => {
		const list = playlist.data;
		if (!list?.isOwner || !videoId || playlistPosition.index < 0) return;
		if (list.lastVideoId !== videoId) void rememberLastPlayed(list.id, videoId);
	});

	function playNext() {
		const next = playlistPosition.next;
		if (next && playback) void goto(playlistWatchHref(next.id, playback.id));
	}

	const explicitStart = $derived(parseStartTime(timeParam));
	const savedProgress = createVideoProgressQuery(() => (explicitStart === null ? videoId : null));
	const reportProgress = createProgressReporter();
	// The player reads the start time once, so it waits until the saved position is known.
	const startKnown = $derived(
		explicitStart !== null ||
			(session.ready && (!session.user || !savedProgress.isPending || !videoId))
	);

	// An explicit ?t= wins over where the user left off.
	const startTime = $derived.by(() => {
		if (!videoId) return 0;
		if (explicitStart !== null) return explicitStart;
		const saved = session.user ? (savedProgress.data ?? null) : readLocalProgress(videoId);
		return resumePosition(saved) ?? 0;
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
		<div
			class="watch"
			data-theater={theater ? '' : undefined}
			data-playlist={playlistStatus ? '' : undefined}
		>
			<div id="player" class="watch-player">
				{#if problem}
					<PlayerMessage
						{problem}
						{videoId}
						{poster}
						onretry={problem.retry ? retry : undefined}
						class="watch-frame"
					/>
				{:else if details && startKnown}
					{#key videoId}
						<Player
							bind:this={player}
							src={manifestUrl(videoId)}
							{poster}
							{subtitles}
							storyboard={details.storyboard_url ?? undefined}
							{startTime}
							{theater}
							autoplay
							ontheatertoggle={toggleTheater}
							onprogress={reportProgress(videoId)}
							onended={playlistPosition.next ? playNext : undefined}
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
					<VideoDetails video={details} bind:descriptionExpanded onseek={seek}>
						{#snippet actions()}<SaveToPlaylistButton {videoId} />{/snippet}
					</VideoDetails>
				{:else if video.isPending}
					<VideoDetailsSkeleton />
				{/if}
			</div>

			{#if playlistStatus}
				<PlaylistPanel
					playlist={playback}
					currentId={videoId}
					status={playlistStatus}
					hasMore={!localList && youtubePlaylist.hasNextPage}
					loadingMore={youtubePlaylist.isFetchingNextPage}
					loadFailed={youtubePlaylist.isFetchNextPageError}
					onloadmore={() => void youtubePlaylist.fetchNextPage()}
					class="watch-playlist"
				/>
			{/if}

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

	.watch[data-playlist] {
		grid-template-areas: 'player' 'playlist' 'meta' 'related';
	}

	.watch :global(.watch-playlist) {
		grid-area: playlist;
		max-height: 26rem;
	}

	/* A notice instead of a list only takes the space it needs. */
	.watch :global(.watch-playlist:not([data-status='ready'], [data-status='loading'])) {
		align-self: start;
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

		.watch[data-playlist] {
			grid-template-areas: 'player playlist' 'meta related';
		}

		/* Beside the player, the panel takes the player's height instead of adding its own. */
		.watch[data-playlist]:not([data-theater]) :global(.watch-playlist[data-status='ready']) {
			height: 0;
			min-height: 100%;
			max-height: none;
		}

		.watch[data-playlist][data-theater] {
			grid-template-rows: auto auto 1fr;
			grid-template-areas: 'player player' 'meta playlist' 'meta related';
		}
	}

	/* Single column: the player runs full-bleed under the header, like theater mode. */
	@container (width < 60rem) {
		.watch {
			padding-top: 0;
		}

		.watch .watch-player {
			margin-inline: calc(var(--page-gutter) * -1);
			background: oklch(0.13 0.004 286);
		}

		.watch :global(.watch-frame .oxi-player-frame) {
			--media-video-border-radius: 0;
		}

		.watch :global(.watch-frame .oxi-player-frame)::after {
			border-color: transparent;
		}

		.watch :global(.watch-frame [data-slot='skeleton']),
		.watch :global(.watch-frame[data-testid='player-message']) {
			border-radius: 0;
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
