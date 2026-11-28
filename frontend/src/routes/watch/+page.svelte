<script lang="ts">
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { ApiError } from '#lib/api/mapping.js';
	import { createVideoProgressQuery } from '#lib/api/history.js';
	import { createLikedVideosQuery, mergeLikedVideos } from '#lib/api/likes.js';
	import {
		isLikedListId,
		isLocalPlaylistId,
		isMixId,
		needsMoreItems,
		neighbors
	} from '#lib/api/playlist-order.js';
	import { createPlaylistQuery, useLastPlayed } from '#lib/api/playlists.js';
	import { useManifest, useVideo } from '#lib/api/video.js';
	import { createYouTubePlaylistQuery, mergeYouTubePlaylist } from '#lib/api/youtube-playlists.js';
	import { playlistWatchHref } from '#lib/components/playlist/links.js';
	import {
		likedPlayback,
		localPlayback,
		youtubePlayback
	} from '#lib/components/playlist/playback.js';
	import PlaylistPanel from '#lib/components/playlist/playlist-panel.svelte';
	import SaveToPlaylistButton from '#lib/components/playlist/save-to-playlist-button.svelte';
	import AddToPartyButton from '#lib/components/together/add-to-party-button.svelte';
	import { addErrorMessage } from '#lib/components/together/errors.js';
	import PartyElsewhere from '#lib/components/together/party-elsewhere.svelte';
	import StartPartyButton from '#lib/components/together/start-party-button.svelte';
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
	import WatchPanel from '#lib/components/watch/watch-panel.svelte';
	import { videoPoster, watchMedia } from '#lib/components/watch/watch-media.js';
	import UpNext from '#lib/components/queue/up-next.svelte';
	import { createSettingsQuery, playbackPreferences } from '#lib/api/settings.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';

	const THEATER_KEY = 'oxistream:theater';
	// Enough to find a video anywhere in YouTube's largest playlists (5,000 videos).
	const MAX_PLAYLIST_PAGES = 60;

	const videoId = $derived(page.url.searchParams.get('v'));
	const timeParam = $derived(page.url.searchParams.get('t'));

	const listId = $derived(page.url.searchParams.get('list'));

	const video = useVideo(() => videoId);
	const playlist = createPlaylistQuery(() => listId);
	const youtubePlaylist = createYouTubePlaylistQuery(() => listId);
	const likedVideos = createLikedVideosQuery(() => isLikedListId(listId));
	const rememberLastPlayed = useLastPlayed();
	const manifest = useManifest(() => videoId);
	const settings = createSettingsQuery();

	let theater = $state(readJson(THEATER_KEY, (v): v is boolean => typeof v === 'boolean') ?? false);
	let descriptionExpanded = $state(false);

	const details = $derived(video.data);
	const problem = $derived.by(() => {
		if (video.isError) return describeProblem(video.error);
		if (details?.is_live) return liveProblem();
		if (manifest.isError) return describeProblem(manifest.error);
		return null;
	});
	const poster = $derived(details ? videoPoster(details) : undefined);
	const related = $derived(details ? playableRelated(details.related) : []);
	const title = $derived(
		details
			? pageTitle(details.title)
			: video.isError
				? pageTitle(problem?.title)
				: pageTitle(m.watch_title())
	);

	const localList = $derived(isLocalPlaylistId(listId));
	const likedList = $derived(isLikedListId(listId));
	const youtubeList = $derived(mergeYouTubePlaylist(youtubePlaylist.data));
	const liked = $derived(mergeLikedVideos(likedVideos.data));
	// Paged lists load more items as playback gets close to the end of what is loaded.
	const pagedList = $derived(localList ? undefined : likedList ? likedVideos : youtubePlaylist);
	const playback = $derived.by(() => {
		if (localList) return playlist.data ? localPlayback(playlist.data) : undefined;
		if (likedList) {
			return liked && session.user ? likedPlayback(liked, session.user.username) : undefined;
		}
		return youtubeList ? youtubePlayback(youtubeList, youtubePlaylist.hasNextPage) : undefined;
	});
	const playlistPosition = $derived(playback ? neighbors(playback.items, videoId) : { index: -1 });
	const playlistStatus = $derived.by(() => {
		if (!listId) return null;
		if (isMixId(listId)) return 'mix';
		if (playback) return 'ready';
		if (likedList && session.ready && !session.user) return 'unavailable';
		const error = localList ? playlist.error : pagedList?.error;
		if (!error) return 'loading';
		return error instanceof ApiError && error.reason === 'mix' ? 'mix' : 'unavailable';
	});

	// Load pages until the current video and the one after it are known.
	$effect(() => {
		const query = pagedList;
		if (!query || !playback || !query.hasNextPage || query.isFetching) return;
		if (query.isFetchNextPageError) return;
		if ((query.data?.pages.length ?? 0) >= MAX_PLAYLIST_PAGES) return;
		if (needsMoreItems(playback.items, videoId)) void query.fetchNextPage();
	});

	// The owner's resume point for "Continue" on the playlist page.
	$effect(() => {
		const list = playlist.data;
		if (!list?.isOwner || !videoId || playlistPosition.index < 0) return;
		if (list.lastVideoId !== videoId) void rememberLastPlayed(list.id, videoId);
	});

	// Where the player continues on this page once the queue is empty, and goes back to.
	const playlistNext = $derived(
		playlistPosition.next && playback && !party.active
			? playlistWatchHref(playlistPosition.next.id, playback.id)
			: undefined
	);
	const playlistPrevious = $derived(
		playlistPosition.previous && playback && !party.active
			? playlistWatchHref(playlistPosition.previous.id, playback.id)
			: undefined
	);

	const explicitStart = $derived(parseStartTime(timeParam));
	const savedProgress = createVideoProgressQuery(() => (explicitStart === null ? videoId : null));
	// The player reads the start time once, so it waits until the saved position is known.
	const startKnown = $derived(
		explicitStart !== null ||
			(session.ready && (!session.user || !savedProgress.isPending || !videoId))
	);
	// Playback preferences are read once, too.
	const settingsKnown = $derived(session.ready && (!session.user || !settings.isPending));

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
			if (seconds !== null) appPlayer.seek(seconds);
		}
		lastTime = t ?? '';
	});

	$effect(() => {
		void videoId;
		descriptionExpanded = false;
	});

	// The app-wide player keeps playing across pages; this page only loads it for a new video.
	const playing = $derived(videoId !== null && appPlayer.media?.videoId === videoId);
	/**
	 * Chapters and transcript seek like the player: in a party only for members who may control
	 * it, since others would be snapped back.
	 */
	const canSeek = $derived(playing && (!party.active || party.canControl));
	/**
	 * The party holds the player and plays another video (or none yet), so this page cannot
	 * load its own; the party's video goes on in the mini player.
	 */
	const partyElsewhere = $derived(
		videoId !== null && party.active && !playing && party.current?.video.id !== videoId
	);

	/** What this tab last asked the party to do with this video. */
	let partyRequest = $state<{ action: 'play' | 'queue'; at: number } | null>(null);
	$effect(() => {
		void [videoId, party.id];
		partyRequest = null;
	});
	const partyQueued = $derived.by(() => {
		if (!party.room?.queue.some((e) => e.video.id === videoId)) return null;
		return partyRequest?.action === 'queue' ? 'added' : 'already';
	});
	const partyError = $derived.by(() => {
		const last = party.connection?.lastError;
		return partyRequest && last && last.at >= partyRequest.at ? addErrorMessage(last.code) : null;
	});
	// Waits for the switch or an error; a reconnect allows trying again.
	const partyBusy = $derived(
		party.status !== 'open' ||
			(partyRequest?.action === 'play' && !partyError) ||
			(partyRequest?.action === 'queue' && !partyError && !partyQueued)
	);
	$effect(() => {
		if (party.status !== 'open') partyRequest = null;
	});

	function partyRequestSend(action: 'play' | 'queue') {
		if (!videoId) return;
		partyRequest = { action, at: Date.now() };
		if (action === 'play') party.playNow(videoId);
		else party.add(videoId);
	}

	const ready = $derived(
		videoId !== null && !problem && details?.id === videoId && startKnown && settingsKnown
	);

	$effect(() => {
		if (!videoId || playing || appPlayer.claim) return;
		// Another video stops right away instead of playing on while this one loads.
		if (appPlayer.media) appPlayer.stop();
		if (!ready || !details) return;
		appPlayer.load(
			watchMedia(details, {
				startTime,
				playback: playbackPreferences(settings.data),
				href: watchHref(page.url.searchParams)
			})
		);
	});

	/** This page without `t`, so coming back from the mini player keeps the position. */
	function watchHref(params: Iterable<[string, string]>) {
		const query = [...params].filter(([key]) => key !== 't');
		return `${resolve('/watch')}?${new URLSearchParams(query)}`;
	}

	// Back on the page of the video that is already playing: only an explicit ?t= moves it.
	onMount(() => {
		if (playing && explicitStart !== null) appPlayer.seek(explicitStart);
	});

	function toggleTheater() {
		theater = !theater;
		writeJson(THEATER_KEY, theater);
	}

	function seek(seconds: number) {
		appPlayer.seek(seconds);
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
				{#if problem && !(party.active && playing)}
					<PlayerMessage
						{problem}
						{videoId}
						{poster}
						onretry={problem.retry ? retry : undefined}
						class="watch-frame"
					/>
				{:else if playing}
					<div
						class="watch-frame watch-slot"
						data-testid="player-slot"
						{@attach appPlayer.slot(() => ({
							theater,
							ontheatertoggle: toggleTheater,
							next: playlistNext,
							previous: playlistPrevious
						}))}
					></div>
				{:else if partyElsewhere}
					<PartyElsewhere
						{poster}
						playing={party.current?.video.title}
						queued={partyQueued}
						disabled={partyBusy}
						error={partyError}
						onplaynow={videoId && party.canPlayNow(videoId)
							? () => partyRequestSend('play')
							: undefined}
						onqueue={party.canAdd ? () => partyRequestSend('queue') : undefined}
						class="watch-frame"
					/>
				{:else}
					<div class="watch-frame" role="status" aria-label={m.watch_loading()}>
						<Skeleton class="aspect-video w-full rounded-xl" />
					</div>
				{/if}
			</div>

			<div class="watch-meta">
				{#if details}
					<VideoDetails video={details} bind:descriptionExpanded onseek={seek}>
						{#snippet actions()}<SaveToPlaylistButton {videoId} /><AddToPartyButton
								{videoId}
							/><StartPartyButton {videoId} />{/snippet}
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
					hasMore={pagedList?.hasNextPage ?? false}
					loadingMore={pagedList?.isFetchingNextPage ?? false}
					loadFailed={pagedList?.isFetchNextPageError ?? false}
					onloadmore={() => void pagedList?.fetchNextPage()}
					class="watch-playlist"
				/>
			{/if}

			<div class="watch-side">
				{#if details}
					<WatchPanel
						video={details}
						time={playing ? appPlayer.time : null}
						showing={playing ? appPlayer.captions : null}
						onseek={canSeek ? (seconds) => appPlayer.seek(seconds) : undefined}
					/>
				{/if}
				{#if !party.active && appPlayer.queue.length > 0}
					<UpNext />
				{/if}
				{#if details || video.isPending}
					<RelatedVideos videos={related} loading={video.isPending} />
				{/if}
			</div>
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

	.watch-slot {
		aspect-ratio: 16 / 9;
	}

	.watch-meta {
		grid-area: meta;
		min-width: 0;
	}

	.watch-side {
		grid-area: related;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
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

		.watch-side {
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

		/* Read by the app player, which draws over this slot. */
		.watch .watch-slot {
			--oxi-slot-flush: 1;
		}

		.watch :global(.watch-frame [data-slot='skeleton']),
		.watch :global(.watch-frame[data-testid='player-message']),
		.watch :global(.watch-frame[data-testid='party-elsewhere']) {
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
		.watch[data-theater] :global(.watch-frame[data-testid='player-message']),
		.watch[data-theater] :global(.watch-frame[data-testid='party-elsewhere']) {
			border-radius: 0;
		}
	}
</style>
