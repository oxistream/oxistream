import type { Snippet } from 'svelte';
import type { Attachment } from 'svelte/attachments';
import type { PlayerSubtitle } from '#lib/components/player/player.svelte';
import type { PlaybackPreferences } from '#lib/components/player/playback.js';
import type { WatchProblem } from '#lib/components/watch/errors.js';
import { UpNextQueue, type QueueVideo } from './queue.svelte.js';

/** A video loaded into the app-wide player. */
export type PlayerMedia = {
	videoId: string;
	title: string;
	channel?: { id: string; name: string } | null;
	poster?: string;
	subtitles?: PlayerSubtitle[];
	storyboard?: string;
	/** Seconds to start from; a function is read once the media can seek. */
	startTime?: number | (() => number);
	/** Default `true`. */
	autoplay?: boolean;
	playback?: PlaybackPreferences;
	/** Where the mini player leads back to, e.g. the watch page with its playlist. */
	href: string;
	/** Reports watch progress and resume points. Default `true`. */
	trackProgress?: boolean;
	/** Skips SponsorBlock segments set to auto; `false` only offers the skip button. Default `true`. */
	autoSkip?: boolean;
	/** The speed is set with `setRate` instead of the saved preference, e.g. to stay in sync. */
	externalRate?: boolean;
	/** Media events for whoever loaded the video, e.g. to keep a party in sync. */
	events?: PlayerEvents;
	/** Shown instead of the video, e.g. when a party's video cannot play. */
	problem?: WatchProblem;
	onretry?: () => void;
};

export type PlayerEvents = {
	onplay?: (currentTime: number) => void;
	onpause?: (currentTime: number) => void;
	onseeked?: (currentTime: number) => void;
	ontimeupdate?: (currentTime: number, duration: number) => void;
	onended?: () => void;
};

/** What the page showing the player in its slot adds, read while the player sits there. */
export type SlotOptions = {
	theater?: boolean;
	ontheatertoggle?: () => void;
	/** Where to go after this video once the queue is empty, e.g. the next playlist video. */
	next?: string;
	/** Where "previous" leads, e.g. the playlist video before this one. */
	previous?: string;
};

/** The calls the mounted player component offers. */
export type PlayerControls = {
	seek(seconds: number): void;
	play(): void;
	pause(): void;
	setTime(seconds: number): void;
	position(): number;
	setRate(rate: number): void;
	mediaElement(): HTMLVideoElement | undefined;
	isPaused(): boolean;
};

/**
 * Exclusive use of the player, e.g. by a watch party: while held, pages cannot load other
 * videos, and closing the mini player asks the holder instead of stopping.
 */
export type PlayerClaim = {
	readonly owner: string;
	load(media: PlayerMedia): void;
	/** Gives the player back; playback goes on until someone stops it. */
	release(): void;
};

export type ClaimOptions = {
	/** Replaces stopping when the mini player is closed, e.g. to confirm leaving the party. */
	onclose?: () => void;
	/** Label of the mini player's close button. */
	closeLabel?: () => string;
	/** Drawn over the video, e.g. a party's sync readout. */
	overlay?: Snippet;
	/** Replace the up-next queue for `next()` and `previous()` while held. */
	onnext?: () => void;
	onprevious?: () => void;
};

/** Going back further than this into a video restarts it instead. */
const RESTART_SECONDS = 3;
const MAX_HISTORY = 50;

/**
 * The one player of the app. The root layout renders it; a page shows it in place with
 * `slot`, and without a slot it continues as the mini player, so navigation never reloads it.
 */
export class AppPlayer {
	media = $state.raw<PlayerMedia | null>(null);
	/** Bumped on every load, so loading the same video again starts over. */
	generation = $state(0);
	slotElement = $state.raw<HTMLElement | null>(null);
	slotOptions = $state.raw<() => SlotOptions>(() => ({}));
	paused = $state(true);
	/** The video played to its end and nothing followed. */
	ended = $state(false);
	claim = $state.raw<(PlayerClaim & ClaimOptions) | null>(null);
	controls: PlayerControls | undefined;
	/** The element around the player, wherever it is shown. */
	element = $state.raw<HTMLElement | null>(null);
	readonly queue: UpNextQueue;
	/** A queued video to load outside the watch page; the host fetches its details. */
	pending = $state.raw<QueueVideo | null>(null);
	/** Opens a page, e.g. the watch page of the next video; set by the host. */
	navigate: (href: string) => void = () => {};
	/** Builds the watch page link of a video; set by the host. */
	watchHref: (videoId: string) => string = (id) => `/watch?v=${id}`;

	/** Where to go once the queue is empty, e.g. back to the playlist it interrupted. */
	#resume = $state.raw<string | null>(null);
	/** The video `next()` or `previous()` asked for, so its load keeps `#resume`. */
	#expect: string | null = null;
	#history: QueueVideo[] = [];
	#last: QueueVideo | null = null;
	#back = false;

	constructor(queue = new UpNextQueue()) {
		this.queue = queue;
	}

	/** Whether a page shows the player in its slot. */
	get inline() {
		return this.media !== null && this.slotElement !== null;
	}

	/** Loads a video unless the player is claimed. Returns whether it did. */
	load(media: PlayerMedia): boolean {
		if (this.claim) return false;
		if (media.videoId !== this.#expect) this.#resume = null;
		this.#expect = null;
		this.pending = null;
		this.#load(media);
		return true;
	}

	#load(media: PlayerMedia) {
		if (!this.claim) {
			const last = this.#last;
			if (last && last.id !== media.videoId && !this.#back) {
				this.#history = [...this.#history, last].slice(-MAX_HISTORY);
			}
			this.#back = false;
			this.#last = {
				id: media.videoId,
				title: media.title,
				channel: media.channel,
				thumbnailUrl: media.poster
			};
		}
		this.media = media;
		this.paused = true;
		this.ended = false;
		this.generation++;
	}

	/** Ends playback and removes the player, also the mini player. */
	stop() {
		this.media = null;
		this.paused = true;
		this.controls = undefined;
	}

	/** The mini player's close button. */
	close() {
		if (this.claim?.onclose) return this.claim.onclose();
		this.stop();
		this.pending = null;
		this.#resume = null;
		this.#last = null;
	}

	/** Whether `next()` has somewhere to go. */
	get hasNext(): boolean {
		if (this.claim) return this.claim.onnext !== undefined;
		if (this.queue.length > 0) return true;
		return this.inline && (this.slotOptions().next ?? this.#resume) != null;
	}

	/** Whether `previous()` does anything; it can always restart the current video. */
	get hasPrevious(): boolean {
		if (this.claim) return this.claim.onprevious !== undefined;
		return this.media !== null;
	}

	/**
	 * Plays what comes next: the queue first, then where the page continues (the next playlist
	 * video), but only while the player is shown on that page. Returns whether it went anywhere.
	 */
	next(): boolean {
		if (this.claim) {
			this.claim.onnext?.();
			return this.claim.onnext !== undefined;
		}
		const after = this.inline ? this.slotOptions().next : undefined;
		const entry = this.queue.shift();
		if (entry) {
			if (after) this.#resume = after;
			this.#open(entry);
			return true;
		}
		const href = after ?? (this.inline ? this.#resume : null);
		if (!href) return false;
		this.#resume = null;
		this.#expect = null;
		this.navigate(href);
		return true;
	}

	/**
	 * Restarts the video once it has played a few seconds, else goes back: to the page's
	 * previous video (in a playlist), or to the one played before, queueing the current one next.
	 */
	previous(): boolean {
		if (this.claim) {
			this.claim.onprevious?.();
			return this.claim.onprevious !== undefined;
		}
		if (!this.media) return false;
		if (this.position() > RESTART_SECONDS) {
			this.seek(0);
			return true;
		}
		const before = this.inline ? this.slotOptions().previous : undefined;
		if (before) {
			this.navigate(before);
			return true;
		}
		const last = this.#history.at(-1);
		if (!last) {
			this.seek(0);
			return true;
		}
		this.#history = this.#history.slice(0, -1);
		if (this.#last) this.queue.addNext(this.#last);
		this.#back = true;
		this.#open(last);
		return true;
	}

	/** Queues a video, at the front with `next`. Plays it right away when nothing plays. */
	enqueue(video: QueueVideo, { next = false } = {}) {
		if (next) this.queue.addNext(video);
		else this.queue.add(video);
		if ((!this.media || this.ended) && !this.claim && !this.pending) this.next();
	}

	/** Plays a queued entry now, taking it off the queue. */
	playEntry(key: string) {
		if (this.claim) return;
		const entry = this.queue.take(key);
		if (!entry) return;
		const after = this.inline ? this.slotOptions().next : undefined;
		if (after) this.#resume = after;
		this.#open(entry);
	}

	/** On the watch page, opens that page for the video; elsewhere loads it in the mini player. */
	#open(video: QueueVideo) {
		this.#expect = video.id;
		if (this.inline) {
			this.navigate(this.watchHref(video.id));
		} else {
			this.pending = video;
		}
	}

	/** Takes the player for `owner`. Fails while someone else holds it. */
	take(owner: string, options: ClaimOptions = {}): PlayerClaim | null {
		if (this.claim && this.claim.owner !== owner) return null;
		const claim: PlayerClaim & ClaimOptions = {
			owner,
			...options,
			load: (media) => {
				if (this.claim === claim) this.#load(media);
			},
			release: () => {
				if (this.claim === claim) this.claim = null;
			}
		};
		this.claim = claim;
		return claim;
	}

	/**
	 * Shows the player in the element instead of the mini player. The element sets the size
	 * (16:9); `options` is read reactively for as long as the player stays there.
	 */
	slot(options: () => SlotOptions = () => ({})): Attachment<HTMLElement> {
		return (node) => {
			this.slotElement = node;
			this.slotOptions = options;
			return () => {
				if (this.slotElement !== node) return;
				this.slotElement = null;
				this.slotOptions = () => ({});
			};
		};
	}

	/** Jumps to `seconds` and keeps playing. */
	seek(seconds: number) {
		this.controls?.seek(seconds);
	}

	play() {
		this.controls?.play();
	}

	pause() {
		this.controls?.pause();
	}

	toggle() {
		if (this.paused) this.play();
		else this.pause();
	}

	/** Jumps to `seconds` without changing whether it plays. */
	setTime(seconds: number) {
		this.controls?.setTime(seconds);
	}

	position(): number {
		return this.controls?.position() ?? 0;
	}

	setRate(rate: number) {
		this.controls?.setRate(rate);
	}

	mediaElement(): HTMLVideoElement | undefined {
		return this.controls?.mediaElement();
	}
}

export const appPlayer = new AppPlayer();
