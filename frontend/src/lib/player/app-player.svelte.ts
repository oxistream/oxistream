import type { Snippet } from 'svelte';
import type { Attachment } from 'svelte/attachments';
import type { PlayerSubtitle } from '#lib/components/player/player.svelte';
import type { PlaybackPreferences } from '#lib/components/player/playback.js';
import type { WatchProblem } from '#lib/components/watch/errors.js';

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
	/** Only called while the player is in this slot, e.g. to play the next playlist video. */
	onended?: () => void;
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
};

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
	claim = $state.raw<(PlayerClaim & ClaimOptions) | null>(null);
	controls: PlayerControls | undefined;
	/** The element around the player, wherever it is shown. */
	element = $state.raw<HTMLElement | null>(null);
	/** Width a panel covers at the end of the viewport, which the mini player keeps clear of. */
	endInset = $state('0px');

	/** Whether a page shows the player in its slot. */
	get inline() {
		return this.media !== null && this.slotElement !== null;
	}

	/** Loads a video unless the player is claimed. Returns whether it did. */
	load(media: PlayerMedia): boolean {
		if (this.claim) return false;
		this.#load(media);
		return true;
	}

	#load(media: PlayerMedia) {
		this.media = media;
		this.paused = true;
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
		if (this.claim?.onclose) this.claim.onclose();
		else this.stop();
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
