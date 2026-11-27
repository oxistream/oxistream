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
	import {
		DriftController,
		PAUSED_DRIFT,
		PLAYING_DRIFT,
		SEEK_THRESHOLD,
		livePosition,
		localNow,
		saveRateCost,
		savedRateCost,
		type Playback
	} from '#lib/together/sync.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import SyncReadout, { type SyncStats } from './sync-readout.svelte';

	const CONTROL_MS = 250;
	const AUTOPLAY_GRACE_MS = 1500;
	/** A pause this close to the end is the media stopping before `ended`, not a user's pause. */
	const END_SLACK = 0.75;
	const OWN_SEEK_MS = 3000;
	/** After a seek or a stall, drift counts once playback ran smoothly this long. */
	const SETTLE_MS = 500;
	/** How long the room may take to confirm what this player did before it is followed again. */
	const REPORT_GRACE_MS = 2000;
	/** While paused, a seek that did not land is retried this often. */
	const PAUSED_SEEK_MS = 1000;
	const STUCK_MS = 4000;
	/** A seek this soon after input on the player is the user's. */
	const INPUT_MS = 1000;
	/** Longest a press on the player holds off following the room. */
	const USER_HOLD_MS = 15_000;

	let {
		videoId,
		playback,
		serverNow,
		clockStats,
		debug = false,
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
		clockStats?: () => { offset: number; rtt: number | null };
		/** Shows the sync readout. */
		debug?: boolean;
		/** `at` is the server time at which the position was read. */
		onplay: (position: number, at: number) => void;
		onpause: (position: number) => void;
		onseek: (position: number, at: number) => void;
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
	let playTriedAt = -Infinity;

	const controller = new DriftController(savedRateCost());
	let stalled = false;
	let settleUntil = 0;
	let stats = $state<SyncStats>({
		offset: 0,
		rtt: null,
		drift: null,
		rate: 1,
		lead: controller.lead,
		rateCost: controller.rateCost,
		state: 'idle'
	});

	const target = () => livePosition(playback, serverNow());
	// Seeks made here to follow the room are not reported back as the user's.
	let ownSeekUntil = 0;
	let reportedUntil = 0;
	let pausedSeekAt = -Infinity;
	/** Where this player last sent the media; dash.js sometimes seeks elsewhere on its own. */
	let seekTarget = 0;
	/** When drift was last acted on, or playback last had nothing to correct. */
	let controlledAt = 0;
	let root = $state<HTMLDivElement>();
	let inputAt = -Infinity;
	/** Since when a pointer is down on the player, e.g. dragging the time slider. */
	let pressedAt: number | null = null;
	/** The user is seeking; the room hears of it right away, or on release while dragging. */
	let userSeeking = false;

	/** Read once the media can seek, so loading time does not put this player behind. */
	function startTime(): number {
		const lead = playback.playing ? controller.lead : 0;
		const at = target() + lead;
		seekTarget = at;
		if (at > 0) ownSeekUntil = localNow() + OWN_SEEK_MS;
		if (playback.playing) controller.seeked(localNow());
		return at;
	}

	function seekTo(seconds: number) {
		ownSeekUntil = localNow() + OWN_SEEK_MS;
		// dash.js takes a seek to exactly 0 with everything buffered as a gap and jumps to the end.
		seekTarget = Math.max(seconds, 0.001);
		player?.setTime(seekTarget);
	}

	let endedSent = false;

	function ended() {
		if (endedSent) return;
		endedSent = true;
		onended();
	}

	function report(state: string, drift: number | null = null) {
		if (!debug) return;
		const clock = clockStats?.();
		stats = {
			offset: clock?.offset ?? 0,
			rtt: clock?.rtt ?? null,
			drift: drift === null ? null : drift * 1000,
			rate: controller.rate,
			lead: controller.lead,
			rateCost: controller.rateCost,
			state
		};
	}

	function sync() {
		if (!player) return;
		const now = localNow();
		const position = target();
		// Seeking past the end stalls the player before `ended`; the room is done with this video.
		if (playback.playing && duration > 0 && position >= duration) return ended();
		// What this player just did is on its way to the server; the room follows shortly.
		if (now < reportedUntil) return report('reporting');
		if (pressedAt !== null && now - pressedAt < USER_HOLD_MS) return report('scrubbing');
		if (!playback.playing || player.isPaused()) controlledAt = now;
		if (!playback.playing) {
			blocked = false;
			controller.reset();
			player.setRate(1);
			if (!player.isPaused()) player.pause();
			const drift = player.position() - position;
			// Also while seeking: dash.js may have jumped elsewhere and be stuck there.
			if (Math.abs(drift) > PAUSED_DRIFT && now - pausedSeekAt > PAUSED_SEEK_MS) {
				pausedSeekAt = now;
				seekTo(position);
			}
			return report('paused', drift);
		}
		const media = player.mediaElement();
		const drift = player.position() - position;
		const far = Math.abs(drift) > PLAYING_DRIFT;
		// dash.js sometimes seeks elsewhere on its own, e.g. jumping a gap, and gets stuck there.
		const foreignSeek = !!media?.seeking && Math.abs(player.position() - seekTarget) > 0.5;
		if (far && foreignSeek) {
			seekTo(position + controller.lead);
			controller.seeked(now);
			return report('seeking', drift);
		}
		if (player.isPaused()) {
			controller.reset();
			if (!blocked && now - playTriedAt > AUTOPLAY_GRACE_MS) {
				playTriedAt = now;
				player.play();
				clearTimeout(blockTimer);
				blockTimer = setTimeout(() => {
					if (playback.playing && player?.isPaused()) blocked = true;
				}, AUTOPLAY_GRACE_MS);
			}
			return report(blocked ? 'blocked' : 'starting');
		}
		const waiting = !media || media.seeking || stalled || media.readyState < 3;
		// Far off and unable to play for long, the player is moved on rather than waited for.
		if ((waiting || now < settleUntil) && !(far && now - controlledAt > STUCK_MS)) {
			return report(stalled ? 'buffering' : 'settling', drift);
		}
		controlledAt = now;
		const action = controller.update(drift, now);
		if (action === 'seek') {
			seekTo(position + controller.lead);
			controller.seeked(now);
			player.setRate(1);
			return report('seeking', drift);
		}
		player.setRate(action);
		report(action === 1 ? 'locked' : 'catching up', drift);
	}

	$effect(() => {
		void [playback.playing, playback.position, playback.positionAt, player];
		reportedUntil = 0;
		pausedSeekAt = -Infinity;
		untrack(sync);
	});

	$effect(() => {
		const timer = setInterval(() => untrack(sync), CONTROL_MS);
		return () => clearInterval(timer);
	});

	// Buffering is waited out rather than fought with the playback rate.
	$effect(() => {
		const media = player?.mediaElement();
		if (!media) return;
		const settle = () => (settleUntil = localNow() + SETTLE_MS);
		const waiting = () => {
			stalled = true;
			controller.reset();
			player?.setRate(1);
		};
		const playing = () => {
			stalled = false;
			settle();
		};
		const seeking = () => {
			const now = localNow();
			const own = now < ownSeekUntil && Math.abs(media.currentTime - seekTarget) < 0.5;
			if (own || now - inputAt >= INPUT_MS) return;
			userSeeking = true;
			seekTarget = media.currentTime;
			if (pressedAt === null) reportSeek(seekTarget);
		};
		media.addEventListener('waiting', waiting);
		media.addEventListener('seeking', seeking);
		media.addEventListener('playing', playing);
		media.addEventListener('seeked', settle);
		return () => {
			media.removeEventListener('waiting', waiting);
			media.removeEventListener('seeking', seeking);
			media.removeEventListener('playing', playing);
			media.removeEventListener('seeked', settle);
		};
	});

	$effect(() => {
		const el = root;
		if (!el) return;
		const input = () => (inputAt = localNow());
		const press = () => {
			input();
			pressedAt = inputAt;
		};
		const release = () => {
			if (pressedAt === null) return;
			input();
			pressedAt = null;
			if (userSeeking) reportSeek(seekTarget);
		};
		el.addEventListener('pointerdown', press, true);
		el.addEventListener('keydown', input, true);
		window.addEventListener('pointerup', release, true);
		window.addEventListener('pointercancel', release, true);
		return () => {
			el.removeEventListener('pointerdown', press, true);
			el.removeEventListener('keydown', input, true);
			window.removeEventListener('pointerup', release, true);
			window.removeEventListener('pointercancel', release, true);
		};
	});

	onDestroy(() => {
		clearTimeout(blockTimer);
		saveRateCost(controller.rateCost);
	});

	const reported = () => (reportedUntil = localNow() + REPORT_GRACE_MS);

	function handlePlay(position: number) {
		blocked = false;
		if (playback.playing) return;
		reported();
		onplay(position, serverNow());
	}

	function handlePause(position: number) {
		if (!playback.playing) return;
		if (duration > 0 && position >= duration - END_SLACK) return;
		reported();
		onpause(position);
	}

	function reportSeek(position: number) {
		if (Math.abs(position - target()) <= SEEK_THRESHOLD) return;
		reported();
		onseek(position, serverNow());
	}

	function handleSeeked(position: number) {
		if (userSeeking) {
			if (pressedAt !== null) return;
			userSeeking = false;
			// The room already heard of it, unless the media landed elsewhere.
			if (Math.abs(position - seekTarget) > 0.5) reportSeek(position);
			return;
		}
		if (localNow() < ownSeekUntil) {
			ownSeekUntil = 0;
			return;
		}
		reportSeek(position);
	}

	function join() {
		blocked = false;
		player?.play();
	}
</script>

<div bind:this={root} class={cn('relative', className)} data-testid="room-player">
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
			externalRate
			playback={playbackPreferences(settings.data)}
			onplay={handlePlay}
			onpause={handlePause}
			onseeked={handleSeeked}
			ontimeupdate={(_, total) => (duration = total)}
			onended={ended}
		/>
		{#if debug}
			<SyncReadout {stats} class="absolute start-2 top-2 z-10" />
		{/if}
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
