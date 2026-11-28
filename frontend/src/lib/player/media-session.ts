/** Default step of the seek backward/forward actions, in seconds. */
const SEEK_OFFSET = 10;

export type SessionMedia = {
	title: string;
	artist?: string;
	/** Absolute or relative image URL. */
	artwork?: string;
};

export type SessionActions = {
	play: () => void;
	pause: () => void;
	/** Absent where seeking would be undone, e.g. in a party this user may not control. */
	seekTo?: (seconds: number) => void;
	position?: () => number;
	next?: () => void;
	previous?: () => void;
};

const ACTIONS: MediaSessionAction[] = [
	'play',
	'pause',
	'seekbackward',
	'seekforward',
	'seekto',
	'previoustrack',
	'nexttrack'
];

function session(): MediaSession | undefined {
	return typeof navigator !== 'undefined' && 'mediaSession' in navigator
		? navigator.mediaSession
		: undefined;
}

function setHandler(
	s: MediaSession,
	action: MediaSessionAction,
	handler: MediaSessionActionHandler | null
) {
	try {
		s.setActionHandler(action, handler);
	} catch {
		// The browser does not know this action.
	}
}

/** Shows the video on the lock screen and in media controls; `null` clears it. */
export function setSessionMedia(media: SessionMedia | null, base = document.baseURI) {
	const s = session();
	if (!s) return;
	if (!media || typeof MediaMetadata === 'undefined') {
		s.metadata = null;
		return;
	}
	s.metadata = new MediaMetadata({
		title: media.title,
		artist: media.artist ?? '',
		artwork: media.artwork ? [{ src: new URL(media.artwork, base).href }] : []
	});
}

/** The handlers for `actions`; actions left out get none, so their buttons are hidden. */
export function sessionHandlers(
	actions: SessionActions | null
): Record<MediaSessionAction, MediaSessionActionHandler | null> {
	const handlers = Object.fromEntries(ACTIONS.map((a) => [a, null])) as Record<
		MediaSessionAction,
		MediaSessionActionHandler | null
	>;
	if (!actions) return handlers;
	handlers.play = () => actions.play();
	handlers.pause = () => actions.pause();
	const { seekTo, position } = actions;
	if (seekTo && position) {
		handlers.seekbackward = (d) => seekTo(Math.max(0, position() - (d.seekOffset ?? SEEK_OFFSET)));
		handlers.seekforward = (d) => seekTo(position() + (d.seekOffset ?? SEEK_OFFSET));
		handlers.seekto = (d) => {
			if (d.seekTime !== undefined) seekTo(d.seekTime);
		};
	}
	if (actions.previous) handlers.previoustrack = actions.previous;
	if (actions.next) handlers.nexttrack = actions.next;
	return handlers;
}

export function setSessionActions(actions: SessionActions | null) {
	const s = session();
	if (!s) return;
	for (const [action, handler] of Object.entries(sessionHandlers(actions))) {
		setHandler(s, action as MediaSessionAction, handler);
	}
}

export function setSessionPlaying(playing: boolean | null) {
	const s = session();
	if (!s) return;
	s.playbackState = playing === null ? 'none' : playing ? 'playing' : 'paused';
}

/** A position state browsers accept: finite duration, position within it, positive rate. */
export function positionState(
	position: number,
	duration: number,
	rate: number
): MediaPositionState | null {
	if (!Number.isFinite(duration) || duration <= 0) return null;
	return {
		duration,
		position: Math.min(Math.max(0, Number.isFinite(position) ? position : 0), duration),
		playbackRate: Number.isFinite(rate) && rate > 0 ? rate : 1
	};
}

export function setSessionPosition(state: MediaPositionState | null) {
	const s = session();
	if (!s?.setPositionState) return;
	try {
		s.setPositionState(state ?? undefined);
	} catch {
		// Rejected values only cost the progress shown by the system.
	}
}
