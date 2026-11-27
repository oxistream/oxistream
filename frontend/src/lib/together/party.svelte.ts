import { MediaQuery } from 'svelte/reactivity';
import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { page } from '$app/state';
import { pickImage } from '#lib/api/mapping.js';
import type { VideoDetails } from '#lib/api/video.js';
import type { PlayerSubtitle } from '#lib/components/player/player.svelte';
import type { PlaybackPreferences } from '#lib/components/player/playback.js';
import type { WatchProblem } from '#lib/components/watch/errors.js';
import { m } from '#lib/paraglide/messages.js';
import { appPlayer, type ClaimOptions, type PlayerClaim } from '#lib/player/app-player.svelte.js';
import {
	RoomConnection,
	type ConnectionStatus,
	type RoomClientMessage,
	type RoomQueueEntry
} from './connection.svelte';
import { PartySync } from './party-sync.svelte';
import { guestKey } from './sync';

const OWNER = 'party';
const SESSION_KEY = 'oxistream:party';
const PANEL_KEY = 'oxistream:party-panel';
/** From this width on, the party panel is a sidebar next to the content; below, a sheet. */
export const PARTY_SIDEBAR_QUERY = 'min-width: 64rem';

function readPanelOpen() {
	try {
		return localStorage.getItem(PANEL_KEY) === 'open';
	} catch {
		return false;
	}
}

function writePanelOpen(open: boolean) {
	try {
		localStorage.setItem(PANEL_KEY, open ? 'open' : 'closed');
	} catch {
		// Without storage, the sidebar starts closed.
	}
}

export type PartyEnd = {
	id: string;
	status: Exclude<ConnectionStatus, 'connecting' | 'open' | 'reconnecting'>;
};

export type JoinOptions = {
	/** Joins as a guest with this name; signed-in users join with their account. */
	name?: string;
	/** Shows the sync readout, from `?debug=sync`. */
	debug?: boolean;
	/** Continues where the video played before the party started. */
	resume?: { videoId: string; position: number; playing: boolean };
	/** Rejoining after a reload: a party that is gone is forgotten without a notice. */
	silent?: boolean;
};

type Saved = { id: string; name?: string };

function readSaved(): Saved | null {
	try {
		const value: unknown = JSON.parse(sessionStorage.getItem(SESSION_KEY) ?? 'null');
		if (typeof value !== 'object' || value === null) return null;
		const { id, name } = value as Saved;
		if (typeof id !== 'string' || (name !== undefined && typeof name !== 'string')) return null;
		return { id, name };
	} catch {
		return null;
	}
}

function writeSaved(saved: Saved | null) {
	try {
		if (saved) sessionStorage.setItem(SESSION_KEY, JSON.stringify(saved));
		else sessionStorage.removeItem(SESSION_KEY);
	} catch {
		// Without storage, a reload leaves the party.
	}
}

/**
 * The watch party of this tab. While in one, it holds the app player: the party's video plays
 * in the watch page slot or the mini player, wherever the user browses.
 */
export class Party {
	id = $state<string | null>(null);
	connection = $state.raw<RoomConnection | null>(null);
	sync = $state.raw<PartySync | null>(null);
	debug = $state(false);
	/** The party sidebar on wide screens, remembered per browser. */
	sidebarOpen = $state(readPanelOpen());
	/** The party sheet on narrow screens. */
	sheetOpen = $state(false);
	/** Asks before leaving, e.g. from the mini player. */
	leaveOpen = $state(false);
	/** Why the last party ended for this tab, until the notice is dismissed. */
	ended = $state<PartyEnd | null>(null);
	/** Drawn over the video while in a party, set by the party host. */
	overlay: ClaimOptions['overlay'];

	#claim: (PlayerClaim & ClaimOptions) | null = null;
	/** `undefined` until the first state is shown. */
	#entry: number | null | undefined = undefined;
	#entryVideo: string | null = null;
	/** What the player shows for the current entry: `video` or a problem's title. */
	#shown: string | null = null;
	#resume: JoinOptions['resume'] | null = null;
	#silent = false;
	#wide: MediaQuery | null = null;

	/** Whether the panel is a sidebar next to the content, rather than a sheet. */
	get docked() {
		this.#wide ??= new MediaQuery(PARTY_SIDEBAR_QUERY);
		return this.#wide.current;
	}

	get panelOpen() {
		return this.docked ? this.sidebarOpen : this.sheetOpen;
	}

	set panelOpen(open: boolean) {
		if (!this.docked) {
			this.sheetOpen = open;
			return;
		}
		this.sidebarOpen = open;
		writePanelOpen(open);
	}

	get active() {
		return this.connection !== null;
	}

	get room() {
		return this.connection?.room ?? null;
	}

	get status(): ConnectionStatus {
		return this.connection?.status ?? 'connecting';
	}

	get current() {
		return this.room?.current ?? null;
	}

	get participants() {
		return this.room?.participants ?? [];
	}

	get isOwner() {
		return this.connection?.isOwner ?? false;
	}

	get owner() {
		return this.room?.owner ?? '';
	}

	get inviteUrl() {
		return this.id ? `${location.origin}${resolve('/party/[id]', { id: this.id })}` : '';
	}

	/** Joins `id`, leaving any other party first. */
	join(id: string, options: JoinOptions = {}) {
		if (this.id === id) return;
		this.leave();
		const claim = appPlayer.take(OWNER, {
			onclose: () => (this.leaveOpen = true),
			closeLabel: m.party_leave,
			overlay: this.overlay
		});
		if (!claim) return;
		this.#claim = claim;
		this.#resume = options.resume ?? null;
		this.#silent = options.silent ?? false;
		this.ended = null;
		this.debug = options.debug ?? false;
		this.id = id;
		const name = options.name;
		const connection = new RoomConnection(id, name ? { name, guestKey: guestKey() } : {});
		connection.connect();
		this.connection = connection;
		writeSaved({ id, name });
	}

	/** Joins a party just created with `videoId`, continuing where that video plays now. */
	start(id: string, videoId?: string) {
		const own = videoId !== undefined && !appPlayer.claim && appPlayer.media?.videoId === videoId;
		this.join(id, {
			resume: own
				? { videoId, position: appPlayer.position(), playing: !appPlayer.paused }
				: undefined
		});
		this.panelOpen = true;
	}

	/** Rejoins the party this tab was in before a reload. */
	restore(signedIn: boolean) {
		const saved = readSaved();
		if (!saved || this.id) return;
		if (!signedIn && !saved.name) return writeSaved(null);
		this.join(saved.id, { name: signedIn ? undefined : saved.name, silent: true });
	}

	/** Leaves for good and stops the video for this tab. */
	leave() {
		if (!this.connection) return;
		this.connection.disconnect();
		this.#reset();
	}

	/** The connection ended, e.g. the party was closed or this user removed. */
	finish(status: PartyEnd['status']) {
		const id = this.id;
		if (!this.connection || !id) return;
		this.connection.disconnect();
		const silent = this.#silent && this.room === null;
		this.#reset();
		if (!silent) this.ended = { id, status };
	}

	send(message: Exclude<RoomClientMessage, { type: 'join' | 'ping' }>) {
		this.connection?.send(message);
	}

	add(videoId: string) {
		this.send({ type: 'add', video_id: videoId });
	}

	/** Queues videos in order, as many as fit; the answer lands in `connection.lastAdded`. */
	addMany(videoIds: string[]) {
		this.connection?.addMany(videoIds);
	}

	/** Moves an upcoming entry to index `to` of the queue, unless the room changed meanwhile. */
	move(entry: number, to: number) {
		const room = this.room;
		if (room) this.send({ type: 'move', entry, to, revision: room.revision });
	}

	shuffle() {
		this.send({ type: 'shuffle' });
	}

	/** Plays `videoId` for everyone, taking it from the queue if it is there. */
	playNow(videoId: string) {
		const queued = this.room?.queue.find((e) => e.video.id === videoId);
		if (queued) this.send({ type: 'select', entry: queued.id });
		else this.send({ type: 'add', video_id: videoId, play: true });
	}

	/**
	 * Plays the current entry, called whenever it or its details change. Someone on the watch
	 * page of the previous video follows to the new one; nobody else is navigated. A video that
	 * cannot play shows `problem` instead.
	 */
	show(
		current: RoomQueueEntry | null,
		details: VideoDetails | undefined,
		ready: boolean,
		playback: PlaybackPreferences | undefined,
		problem: { problem: WatchProblem; onretry?: () => void } | null = null
	) {
		const connection = this.connection;
		const claim = this.#claim;
		if (!connection || !claim || !this.room) return;
		const entry = current?.id ?? null;
		if (entry !== this.#entry) {
			const previous = this.#entryVideo;
			this.#entry = entry;
			this.#entryVideo = current?.video.id ?? null;
			this.#shown = null;
			this.sync?.destroy();
			this.sync = null;
			if (!current) {
				appPlayer.stop();
				return;
			}
			appPlayer.pause();
			if (previous && page.route.id === '/watch' && page.url.searchParams.get('v') === previous) {
				void goto(resolve(`/watch?v=${current.video.id}`), { replaceState: true });
			}
		}
		if (!current || !ready) return;
		const video = current.video;
		const media = {
			videoId: video.id,
			title: details?.title ?? video.title,
			channel: details?.channel ?? video.channel,
			poster: pickImage(details?.thumbnails ?? video.thumbnails, 1280),
			href: `${resolve('/watch')}?v=${video.id}`,
			trackProgress: false
		};
		if (problem) {
			const shown = `problem:${problem.problem.title}`;
			if (this.#shown === shown) return;
			this.#shown = shown;
			this.sync?.destroy();
			this.sync = null;
			claim.load({ ...media, autoplay: false, ...problem });
			return;
		}
		if (this.#shown === 'video') return;
		this.#shown = 'video';
		const room = this.room;
		const sync = new PartySync({
			playback: { playing: room.playing, position: room.position, positionAt: room.position_at },
			serverNow: () => connection.serverNow(),
			clockStats: () => connection.clockStats(),
			debug: this.debug,
			onplay: (position, at) => connection.send({ type: 'play', position, at }),
			onpause: (position) => connection.send({ type: 'pause', position }),
			onseek: (position, at) => connection.send({ type: 'seek', position, at }),
			onended: () => connection.send({ type: 'ended', entry: current.id })
		});
		this.sync = sync;
		const subtitles: PlayerSubtitle[] = (details?.subtitles ?? []).map((s) => ({
			id: s.id,
			lang: s.lang,
			url: s.url,
			label: s.auto_generated ? m.player_auto_generated({ label: s.label }) : s.label
		}));
		claim.load({
			...media,
			subtitles,
			storyboard: details?.storyboard_url ?? undefined,
			startTime: sync.startTime,
			autoplay: false,
			playback,
			autoSkip: false,
			externalRate: true,
			events: sync.events
		});
		const resume = this.#resume;
		this.#resume = null;
		// Started from a video that was already playing: the party picks it up from there.
		if (resume?.videoId === video.id && room.position === 0 && !room.playing) {
			const at = connection.serverNow();
			if (resume.playing) connection.send({ type: 'play', position: resume.position, at });
			else if (resume.position > 0) {
				connection.send({ type: 'seek', position: resume.position, at });
			}
		}
	}

	#reset() {
		this.sync?.destroy();
		this.sync = null;
		this.#claim?.release();
		this.#claim = null;
		appPlayer.stop();
		this.connection = null;
		this.id = null;
		this.#entry = undefined;
		this.#entryVideo = null;
		this.#shown = null;
		this.#resume = null;
		this.leaveOpen = false;
		this.sheetOpen = false;
		writeSaved(null);
	}
}

export const party = new Party();
