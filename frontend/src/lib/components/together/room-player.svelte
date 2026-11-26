<script lang="ts">
	import PlayIcon from '@lucide/svelte/icons/play';
	import { onDestroy, untrack } from 'svelte';
	import { pickImage } from '#lib/api/mapping.js';
	import { createSettingsQuery, playbackPreferences } from '#lib/api/settings.js';
	import { manifestUrl, useManifest, useVideo } from '#lib/api/video.js';
	import Player, { type PlayerSubtitle } from '#lib/components/player/player.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { describeProblem, liveProblem } from '#lib/components/watch/errors.js';
	import PlayerMessage from '#lib/components/watch/player-message.svelte';
	import { SEEK_THRESHOLD, livePosition, needsSeek, type Playback } from '#lib/together/sync.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	const DRIFT_CHECK_MS = 2000;
	const AUTOPLAY_GRACE_MS = 1500;
	/** A pause this close to the end is the media stopping before `ended`, not a user's pause. */
	const END_SLACK = 0.75;
	const OWN_SEEK_MS = 3000;

	let {
		videoId,
		playback,
		serverNow,
		onplay,
		onpause,
		onseek,
		onended,
		class: className
	}: {
		/** The player is not reused: mount one per queue entry. */
		videoId: string;
		playback: Playback;
		/** The server's clock in ms since the epoch. */
		serverNow: () => number;
		onplay: (position: number) => void;
		onpause: (position: number) => void;
		onseek: (position: number) => void;
		onended: () => void;
		class?: string;
	} = $props();

	const video = useVideo(() => videoId);
	const manifest = useManifest(() => videoId);
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

	const settings = createSettingsQuery();
	let player = $state<ReturnType<typeof Player>>();
	/** The browser refused to start playback without a click. */
	let blocked = $state(false);
	let duration = 0;
	let blockTimer: ReturnType<typeof setTimeout> | undefined;

	const target = () => livePosition(playback, serverNow());
	// Mounted once per queue entry, starting where the room is.
	const startTime = untrack(target);
	// Seeks made here to follow the room are not reported back as the user's.
	let initialSeek = startTime > 0;
	let ownSeekUntil = 0;

	function seekTo(seconds: number) {
		ownSeekUntil = Date.now() + OWN_SEEK_MS;
		player?.setTime(seconds);
	}

	let endedSent = false;

	function ended() {
		if (endedSent) return;
		endedSent = true;
		onended();
	}

	function sync() {
		if (!player) return;
		const position = target();
		// Seeking past the end stalls the player before `ended`; the room is done with this video.
		if (playback.playing && duration > 0 && position >= duration) return ended();
		if (needsSeek(player.position(), position, playback.playing)) seekTo(position);
		if (playback.playing && player.isPaused()) {
			player.play();
			clearTimeout(blockTimer);
			blockTimer = setTimeout(() => {
				if (playback.playing && player?.isPaused()) blocked = true;
			}, AUTOPLAY_GRACE_MS);
		} else if (!playback.playing) {
			blocked = false;
			if (!player.isPaused()) player.pause();
		}
	}

	$effect(() => {
		void [playback.playing, playback.position, playback.positionAt, player];
		untrack(sync);
	});

	$effect(() => {
		if (!playback.playing) return;
		const timer = setInterval(() => untrack(sync), DRIFT_CHECK_MS);
		return () => clearInterval(timer);
	});

	onDestroy(() => clearTimeout(blockTimer));

	function handlePlay(position: number) {
		blocked = false;
		if (!playback.playing) onplay(position);
	}

	function handlePause(position: number) {
		if (!playback.playing) return;
		if (duration > 0 && position >= duration - END_SLACK) return;
		onpause(position);
	}

	function handleSeeked(position: number) {
		if (initialSeek || Date.now() < ownSeekUntil) {
			initialSeek = false;
			ownSeekUntil = 0;
			return;
		}
		if (Math.abs(position - target()) > SEEK_THRESHOLD) onseek(position);
	}

	function join() {
		blocked = false;
		player?.play();
	}
</script>

<div class={cn('relative', className)} data-testid="room-player">
	{#if problem}
		<PlayerMessage {problem} {videoId} {poster} />
	{:else if details}
		<Player
			bind:this={player}
			src={manifestUrl(videoId)}
			{poster}
			{subtitles}
			storyboard={details.storyboard_url ?? undefined}
			{startTime}
			playback={playbackPreferences(settings.data)}
			onplay={handlePlay}
			onpause={handlePause}
			onseeked={handleSeeked}
			ontimeupdate={(_, total) => (duration = total)}
			onended={ended}
		/>
		{#if blocked}
			<div class="absolute inset-0 flex items-center justify-center rounded-xl bg-black/40">
				<Button size="lg" class="rounded-full px-5" onclick={join}>
					<PlayIcon />
					{m.together_sync_play()}
				</Button>
			</div>
		{/if}
	{:else}
		<div role="status" aria-label={m.watch_loading()}>
			<Skeleton class="aspect-video w-full rounded-xl" />
		</div>
	{/if}
</div>
