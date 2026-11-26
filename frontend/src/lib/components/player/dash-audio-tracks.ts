// The dash.js adapter in @videojs/dash-video 10.7.0 only mirrors video renditions, so the player
// never sees the MPD's audio adaptation sets. This bridge lists them as media audio tracks and
// switches dash.js when one is picked. Remove it once the adapter does this itself.

/** The parts of a dash.js `MediaInfo` this bridge reads. */
type DashTrack = {
	id: string | null;
	lang: string | null;
	labels?: { text: string; lang?: string }[];
	roles?: { value: string }[];
	mimeType?: string | null;
	codec?: string | null;
};

type DashEngine = {
	getTracksFor(type: 'audio'): DashTrack[];
	getCurrentTrackFor(type: 'audio'): DashTrack | null;
	setCurrentTrack(track: DashTrack): void;
	on(event: string, listener: () => void): void;
	off(event: string, listener: () => void): void;
};

type AudioTrackLike = {
	id: string | undefined;
	label: string;
	language: string;
	enabled: boolean;
};

export type AudioTrackHost = EventTarget & {
	readonly engine?: unknown;
	readonly audioTracks: Iterable<AudioTrackLike> & EventTarget;
	addAudioTrack(kind: string, label?: string, language?: string): AudioTrackLike;
	removeAudioTrack(track: AudioTrackLike): void;
};

export type AudioTrackGroup = {
	key: string;
	label: string;
	language: string;
	main: boolean;
	tracks: DashTrack[];
};

/**
 * YouTube offers every language in several codecs (AAC and Opus), each its own adaptation set.
 * Listeners pick a language, not a codec, so sets with the same language and label are one choice.
 */
export function groupAudioTracks(tracks: DashTrack[]): AudioTrackGroup[] {
	const groups = new Map<string, AudioTrackGroup>();
	for (const track of tracks) {
		const language = track.lang ?? '';
		const label = track.labels?.[0]?.text ?? language;
		const key = `${language}|${label}`;
		let group = groups.get(key);
		if (!group) {
			const main = track.roles?.some((role) => role.value === 'main') ?? false;
			group = { key, label, language, main, tracks: [] };
			groups.set(key, group);
		}
		group.tracks.push(track);
	}
	return [...groups.values()];
}

/** Within a language, stay on the codec that is playing so the switch needs no new decoder. */
export function pickTrack(group: AudioTrackGroup, current: DashTrack | null): DashTrack {
	return (
		group.tracks.find((t) => t.mimeType && t.mimeType === current?.mimeType) ?? group.tracks[0]
	);
}

function isEngine(value: unknown): value is DashEngine {
	return (
		typeof value === 'object' &&
		value !== null &&
		'getTracksFor' in value &&
		'setCurrentTrack' in value &&
		'on' in value
	);
}

export function bridgeDashAudioTracks(host: AudioTrackHost): () => void {
	const engine = host.engine;
	if (!isEngine(engine)) return () => {};

	let groups: AudioTrackGroup[] = [];
	let syncing = false;

	const currentKey = () => {
		const current = engine.getCurrentTrackFor('audio');
		return groups.find((g) => g.tracks.includes(current!))?.key;
	};

	const syncEnabled = () => {
		const key = currentKey();
		syncing = true;
		for (const track of host.audioTracks) track.enabled = track.id === key;
		syncing = false;
	};

	const rebuild = () => {
		syncing = true;
		for (const track of [...host.audioTracks]) host.removeAudioTrack(track);
		let tracks: DashTrack[] = [];
		try {
			tracks = engine.getTracksFor('audio');
		} catch {
			// No stream yet; the next streamInitialized rebuilds.
		}
		groups = groupAudioTracks(tracks);
		for (const group of groups) {
			const track = host.addAudioTrack(
				group.main ? 'main' : 'translation',
				group.label,
				group.language
			);
			track.id = group.key;
		}
		syncing = false;
		syncEnabled();
	};

	const onChange = () => {
		if (syncing) return;
		const enabled = [...host.audioTracks].find((t) => t.enabled && t.id !== currentKey());
		const group = groups.find((g) => g.key === enabled?.id);
		if (!group) return;
		engine.setCurrentTrack(pickTrack(group, engine.getCurrentTrackFor('audio')));
	};

	engine.on('streamInitialized', rebuild);
	engine.on('currentTrackChanged', syncEnabled);
	host.audioTracks.addEventListener('change', onChange);
	rebuild();

	return () => {
		engine.off('streamInitialized', rebuild);
		engine.off('currentTrackChanged', syncEnabled);
		host.audioTracks.removeEventListener('change', onChange);
	};
}
