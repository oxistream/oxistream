import type { SyncStats } from '#lib/components/together/sync-readout.svelte';
import {
	appPlayer,
	type PlayerControls,
	type PlayerEvents
} from '#lib/player/app-player.svelte.js';
import type { MemberSync } from './connection.svelte';
import { syncHealth } from './presence';
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
} from './sync';

const CONTROL_MS = 250;
const AUTOPLAY_GRACE_MS = 1500;
/** A pause this close to the end is the media stopping before `ended`, not a user's pause. */
const END_SLACK = 0.75;
const OWN_SEEK_MS = 3000;
/** After a seek or a stall, drift counts once playback ran smoothly this long. */
const SETTLE_MS = 500;
/** How long the party may take to confirm what this player did before it is followed again. */
const REPORT_GRACE_MS = 2000;
/** While paused, a seek that did not land is retried this often. */
const PAUSED_SEEK_MS = 1000;
const STUCK_MS = 4000;
/** A seek this soon after input on the player is the user's. */
const INPUT_MS = 1000;
/** Longest a press on the player holds off following the party. */
const USER_HOLD_MS = 15_000;

export type PartySyncOptions = {
	playback: Playback;
	/** The server's clock in ms since the epoch. */
	serverNow: () => number;
	clockStats?: () => { offset: number; rtt: number | null };
	/** Keeps the sync readout up to date. */
	debug?: boolean;
	/** Whether this user may steer the party; if not, the player snaps back to it instead. */
	canControl?: () => boolean;
	/** `at` is the server time at which the position was read. */
	onplay: (position: number, at: number) => void;
	onpause: (position: number) => void;
	onseek: (position: number, at: number) => void;
	onended: () => void;
};

/**
 * Keeps the app player on the party's position for one queue entry: create one per entry,
 * load the video with its `startTime` and `events`, and destroy it when the entry changes.
 */
export class PartySync {
	/** The browser refused to start playback without a click. */
	blocked = $state(false);
	/** How this player keeps up, for the members list; `null` until it is known. */
	health = $state<MemberSync | null>(null);
	stats = $state<SyncStats>({
		offset: 0,
		rtt: null,
		drift: null,
		rate: 1,
		lead: 0,
		rateCost: 0,
		state: 'idle'
	});

	#options: PartySyncOptions;
	#playback: Playback;
	#controller = new DriftController(savedRateCost());
	/** The player of the previous video, which this one must not steer. */
	#stale: PlayerControls | undefined;
	#timer: ReturnType<typeof setInterval>;
	#blockTimer: ReturnType<typeof setTimeout> | undefined;
	#unbindMedia: (() => void) | undefined;
	#unbindRoot: (() => void) | undefined;
	#media: HTMLVideoElement | undefined;
	#root: HTMLElement | null = null;

	#duration = 0;
	#playTriedAt = -Infinity;
	#stalled = false;
	#settleUntil = 0;
	// Seeks made here to follow the party are not reported back as the user's.
	#ownSeekUntil = 0;
	#reportedUntil = 0;
	#pausedSeekAt = -Infinity;
	/** Where this player last sent the media; dash.js sometimes seeks elsewhere on its own. */
	#seekTarget = 0;
	/** When drift was last acted on, or playback last had nothing to correct. */
	#controlledAt = 0;
	#inputAt = -Infinity;
	/** Since when a pointer is down on the player, e.g. dragging the time slider. */
	#pressedAt: number | null = null;
	/**
	 * The user is seeking, with input on the player or e.g. native controls; the party hears of
	 * it right away, or on release while dragging.
	 */
	#userSeeking: 'input' | 'other' | null = null;
	#endedSent = false;
	#destroyed = false;

	constructor(options: PartySyncOptions) {
		this.#options = options;
		this.#playback = options.playback;
		this.#stale = appPlayer.controls;
		this.stats.lead = this.#controller.lead;
		this.stats.rateCost = this.#controller.rateCost;
		this.#timer = setInterval(() => this.#sync(), CONTROL_MS);
	}

	/** For loading the video. Once destroyed, e.g. the video being paused on the way out is not reported. */
	readonly events: PlayerEvents = {
		onplay: (position) => this.#destroyed || this.#handlePlay(position),
		onpause: (position) => this.#destroyed || this.#handlePause(position),
		onseeked: (position) => this.#destroyed || this.#handleSeeked(position),
		ontimeupdate: (_, total) => (this.#duration = total),
		onended: () => this.#destroyed || this.#ended()
	};

	/** Read once the media can seek, so loading time does not put this player behind. */
	readonly startTime = (): number => {
		this.#bind();
		const playback = this.#playback;
		const lead = playback.playing ? this.#controller.lead : 0;
		const at = this.#target() + lead;
		this.#seekTarget = at;
		if (at > 0) this.#ownSeekUntil = localNow() + OWN_SEEK_MS;
		if (playback.playing) this.#controller.seeked(localNow());
		return at;
	};

	/** The party's playback changed. */
	update(playback: Playback) {
		this.#playback = playback;
		this.#reportedUntil = 0;
		this.#pausedSeekAt = -Infinity;
		this.#sync();
	}

	/** Starts playback after the browser blocked it. */
	join() {
		this.blocked = false;
		this.#player()?.play();
	}

	/** Jumps to where the party is now instead of catching up, and stops holding off for input. */
	resync() {
		const player = this.#player();
		if (!player) return;
		const controller = this.#controller;
		const now = localNow();
		this.#pressedAt = null;
		this.#userSeeking = null;
		this.#reportedUntil = 0;
		controller.reset();
		player.setRate(1);
		const playing = this.#playback.playing;
		this.#seekTo(this.#target() + (playing ? controller.lead : 0));
		if (playing) controller.seeked(now);
		this.#controlledAt = now;
		this.#sync();
	}

	destroy() {
		this.#destroyed = true;
		clearInterval(this.#timer);
		clearTimeout(this.#blockTimer);
		this.#unbindMedia?.();
		this.#unbindRoot?.();
		saveRateCost(this.#controller.rateCost);
	}

	#player(): PlayerControls | undefined {
		const controls = appPlayer.controls;
		return controls === this.#stale ? undefined : controls;
	}

	#target() {
		return livePosition(this.#playback, this.#options.serverNow());
	}

	#seekTo(seconds: number) {
		this.#ownSeekUntil = localNow() + OWN_SEEK_MS;
		// dash.js takes a seek to exactly 0 with everything buffered as a gap and jumps to the end.
		this.#seekTarget = Math.max(seconds, 0.001);
		this.#player()?.setTime(this.#seekTarget);
	}

	#ended() {
		if (this.#endedSent) return;
		this.#endedSent = true;
		this.#options.onended();
	}

	#report(state: string, drift: number | null = null) {
		this.health = syncHealth(state, drift) ?? this.health;
		if (!this.#options.debug) return;
		const clock = this.#options.clockStats?.();
		const controller = this.#controller;
		this.stats = {
			offset: clock?.offset ?? 0,
			rtt: clock?.rtt ?? null,
			drift: drift === null ? null : drift * 1000,
			rate: controller.rate,
			lead: controller.lead,
			rateCost: controller.rateCost,
			state
		};
	}

	#sync() {
		this.#bind();
		const player = this.#player();
		if (!player) return;
		const playback = this.#playback;
		const controller = this.#controller;
		const now = localNow();
		const position = this.#target();
		// Seeking past the end stalls the player before `ended`; the party is done with this video.
		if (playback.playing && this.#duration > 0 && position >= this.#duration) return this.#ended();
		// What this player just did is on its way to the server; the party follows shortly.
		if (now < this.#reportedUntil) return this.#report('reporting');
		if (this.#pressedAt !== null && now - this.#pressedAt < USER_HOLD_MS) {
			return this.#report('scrubbing');
		}
		if (!playback.playing || player.isPaused()) this.#controlledAt = now;
		if (!playback.playing) {
			this.blocked = false;
			controller.reset();
			player.setRate(1);
			if (!player.isPaused()) player.pause();
			const drift = player.position() - position;
			// Also while seeking: dash.js may have jumped elsewhere and be stuck there.
			if (Math.abs(drift) > PAUSED_DRIFT && now - this.#pausedSeekAt > PAUSED_SEEK_MS) {
				this.#pausedSeekAt = now;
				this.#seekTo(position);
			}
			return this.#report('paused', drift);
		}
		const media = player.mediaElement();
		const drift = player.position() - position;
		const far = Math.abs(drift) > PLAYING_DRIFT;
		// dash.js sometimes seeks elsewhere on its own, e.g. jumping a gap, and gets stuck there.
		const foreignSeek = !!media?.seeking && Math.abs(player.position() - this.#seekTarget) > 0.5;
		if (far && foreignSeek) {
			this.#seekTo(position + controller.lead);
			controller.seeked(now);
			return this.#report('seeking', drift);
		}
		if (player.isPaused()) {
			controller.reset();
			if (!this.blocked && now - this.#playTriedAt > AUTOPLAY_GRACE_MS) {
				this.#playTriedAt = now;
				player.play();
				clearTimeout(this.#blockTimer);
				this.#blockTimer = setTimeout(() => {
					if (this.#playback.playing && this.#player()?.isPaused()) this.blocked = true;
				}, AUTOPLAY_GRACE_MS);
			}
			return this.#report(this.blocked ? 'blocked' : 'starting');
		}
		const waiting = !media || media.seeking || this.#stalled || media.readyState < 3;
		// Far off and unable to play for long, the player is moved on rather than waited for.
		if ((waiting || now < this.#settleUntil) && !(far && now - this.#controlledAt > STUCK_MS)) {
			return this.#report(this.#stalled ? 'buffering' : 'settling', drift);
		}
		this.#controlledAt = now;
		const action = controller.update(drift, now);
		if (action === 'seek') {
			this.#seekTo(position + controller.lead);
			controller.seeked(now);
			player.setRate(1);
			return this.#report('seeking', drift);
		}
		player.setRate(action);
		this.#report(action === 1 ? 'locked' : 'catching up', drift);
	}

	/** Listens to the media and the input on the player once they exist, and when they change. */
	#bind() {
		const media = this.#player()?.mediaElement();
		if (media !== this.#media) {
			this.#unbindMedia?.();
			this.#unbindMedia = undefined;
			this.#media = media;
			if (media) this.#unbindMedia = this.#listenMedia(media);
		}
		const root = appPlayer.element;
		if (root !== this.#root) {
			this.#unbindRoot?.();
			this.#unbindRoot = undefined;
			this.#root = root;
			if (root) this.#unbindRoot = this.#listenInput(root);
		}
	}

	// Buffering is waited out rather than fought with the playback rate.
	#listenMedia(media: HTMLVideoElement) {
		const settle = () => (this.#settleUntil = localNow() + SETTLE_MS);
		const waiting = () => {
			this.#stalled = true;
			this.#controller.reset();
			this.#player()?.setRate(1);
		};
		const playing = () => {
			this.#stalled = false;
			settle();
		};
		const seeking = () => {
			const now = localNow();
			const own = now < this.#ownSeekUntil && Math.abs(media.currentTime - this.#seekTarget) < 0.5;
			if (own) return;
			const input = now - this.#inputAt < INPUT_MS;
			// dash.js jumps gaps while stalled, may follow our seeks elsewhere, or jump to the end.
			const engine =
				now < this.#ownSeekUntil ||
				this.#stalled ||
				media.currentTime >= (media.duration || Infinity) - END_SLACK;
			if (!input && engine) return;
			this.#userSeeking = input ? 'input' : 'other';
			this.#seekTarget = media.currentTime;
			if (this.#pressedAt === null) this.#reportSeek(this.#seekTarget, input);
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
	}

	#listenInput(el: HTMLElement) {
		const input = () => (this.#inputAt = localNow());
		const press = () => {
			input();
			this.#pressedAt = this.#inputAt;
		};
		const release = () => {
			if (this.#pressedAt === null) return;
			input();
			this.#pressedAt = null;
			if (this.#userSeeking) this.#reportSeek(this.#seekTarget, this.#userSeeking === 'input');
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
	}

	#reported() {
		this.#reportedUntil = localNow() + REPORT_GRACE_MS;
	}

	#mayControl() {
		return this.#options.canControl?.() ?? true;
	}

	#handlePlay(position: number) {
		this.blocked = false;
		// Not allowed to, the next sync pauses it again.
		if (this.#playback.playing || !this.#mayControl()) return;
		this.#reported();
		this.#options.onplay(position, this.#options.serverNow());
	}

	#handlePause(position: number) {
		if (!this.#playback.playing) return;
		if (this.#duration > 0 && position >= this.#duration - END_SLACK) return;
		if (!this.#mayControl()) {
			// Resumed once the pause is through; a play sent earlier would be lost to it.
			this.#playTriedAt = localNow();
			this.#player()?.play();
			return;
		}
		this.#reported();
		this.#options.onpause(position);
	}

	/** Small seeks without input on the player are noise; with input they are followed. */
	#reportSeek(position: number, input = false) {
		if (!input && Math.abs(position - this.#target()) <= SEEK_THRESHOLD) return;
		if (!this.#mayControl()) return this.resync();
		this.#reported();
		this.#options.onseek(position, this.#options.serverNow());
	}

	#handleSeeked(position: number) {
		if (this.#userSeeking) {
			if (this.#pressedAt !== null) return;
			const input = this.#userSeeking === 'input';
			this.#userSeeking = null;
			// The party already heard of it, unless the media landed elsewhere.
			if (Math.abs(position - this.#seekTarget) > 0.5) this.#reportSeek(position, input);
			return;
		}
		if (localNow() < this.#ownSeekUntil) {
			this.#ownSeekUntil = 0;
			return;
		}
		this.#reportSeek(position);
	}
}
