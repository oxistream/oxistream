export type VideoCodec = 'av1' | 'vp9' | 'h264';

/** Codec and quality preferences from the user's settings. */
export type PlaybackPreferences = {
	/** Most preferred first; empty leaves the choice to dash.js. */
	codecs: readonly VideoCodec[];
	/** Highest rendition, by its shorter side; `null` for no limit. */
	maxQuality: number | null;
	/** Rendition to start with instead of adaptive quality. */
	defaultQuality: number | null;
	/** Language code of the audio to start with; `null` for the original audio. */
	audioLanguage: string | null;
};

export type TrackRole = { schemeIdUri?: string | null; value: string };

/** The parts of a dash.js `MediaInfo` the initial track selection reads. */
export type TrackInfo = {
	type?: string | null;
	codec?: string | null;
	lang?: string | null;
	selectionPriority?: number;
	representationCount?: number;
	roles?: TrackRole[] | null;
	bitrateList?: { bandwidth: number; width?: number; height?: number }[];
};

const DASH_ROLE_SCHEME = 'urn:mpeg:dash:role:2011';
/** The backend marks auto-dubbed sets with this scheme, as DASH has no role for them. */
const OXISTREAM_ROLE_SCHEME = 'urn:oxistream:dash:role:2026';

export type AudioKind = 'original' | 'dubbed' | 'auto-dubbed' | 'other';

export function audioKind(roles: readonly TrackRole[] | null | undefined): AudioKind {
	const has = (scheme: string, value: string) =>
		roles?.some((r) => (r.schemeIdUri ?? DASH_ROLE_SCHEME) === scheme && r.value === value) ??
		false;
	if (has(OXISTREAM_ROLE_SCHEME, 'auto-dub')) return 'auto-dubbed';
	if (has(DASH_ROLE_SCHEME, 'main')) return 'original';
	if (has(DASH_ROLE_SCHEME, 'dub')) return 'dubbed';
	return 'other';
}

const primary = (lang: string | null | undefined) => lang?.split('-')[0].toLowerCase() ?? '';

/**
 * The audio to start with. `null` asks for the original. A language prefers its original or
 * human dub, then falls back to the original, then any track that is not auto-dubbed; an
 * auto-dub only plays if nothing else exists, as the user can still pick one in the menu.
 */
export function selectAudio<T extends TrackInfo>(tracks: T[], language: string | null): T[] {
	const notAuto = tracks.filter((t) => audioKind(t.roles) !== 'auto-dubbed');
	const originals = tracks.filter((t) => audioKind(t.roles) === 'original');
	if (language) {
		const wanted = primary(language);
		const inLanguage = notAuto.filter((t) => primary(t.lang) === wanted);
		const exact = inLanguage.filter((t) => t.lang?.toLowerCase() === language.toLowerCase());
		const matches = exact.length > 0 ? exact : inLanguage;
		const original = matches.filter((t) => audioKind(t.roles) === 'original');
		if (original.length > 0) return original;
		if (matches.length > 0) return best(matches, (t) => t.selectionPriority ?? 1);
	}
	if (originals.length > 0) return originals;
	if (notAuto.length > 0) return best(notAuto, (t) => t.selectionPriority ?? 1);
	if (language) {
		const autoInLanguage = tracks.filter((t) => primary(t.lang) === primary(language));
		if (autoInLanguage.length > 0) return autoInLanguage;
	}
	return best(tracks, (t) => t.selectionPriority ?? 1);
}

/** The parts of a parsed MPD `Representation` the quality limit reads; audio has no size. */
export type RepresentationInfo = { width?: number | string; height?: number | string };

/** dash.js codec strings look like `video/webm;codecs="vp09.00.51.08"` or just `avc1.64001f`. */
export function codecFamily(codec: string | null | undefined): VideoCodec | undefined {
	const value = (/codecs="?([^"]+)/i.exec(codec ?? '')?.[1] ?? codec ?? '').trim().toLowerCase();
	if (value.startsWith('av01')) return 'av1';
	if (value.startsWith('vp09') || value.startsWith('vp9')) return 'vp9';
	if (value.startsWith('avc1') || value.startsWith('avc3')) return 'h264';
	return undefined;
}

function best<T>(items: T[], score: (item: T) => number): T[] {
	const top = Math.max(...items.map(score));
	return items.filter((item) => score(item) === top);
}

const average = (values: number[]) =>
	values.length > 0 ? values.reduce((a, b) => a + b, 0) / values.length : 0;

/**
 * dash.js's default among equals once a custom selection is set, which takes the first track:
 * the lowest bitrate per pixel (per track for audio), then the highest bitrate.
 */
function mostEfficient<T extends TrackInfo>(tracks: T[]): T[] {
	if (tracks.length < 2) return tracks;
	const cost = (track: T) =>
		average(
			(track.bitrateList ?? []).map(
				(b) => b.bandwidth / Math.max(1, (b.width ?? 0) * (b.height ?? 0))
			)
		);
	const peak = (track: T) => Math.max(0, ...(track.bitrateList ?? []).map((b) => b.bandwidth));
	return best(
		best(tracks, (track) => -cost(track)),
		peak
	);
}

/**
 * Replaces dash.js's initial track selection: video tracks by codec preference, the widest
 * range first among equals; audio by `selectAudio`; other tracks as dash.js does by default,
 * by `selectionPriority` and then the main role.
 */
export function trackSelector(codecs: readonly VideoCodec[], audioLanguage: string | null = null) {
	return <T extends TrackInfo>(tracks: T[]): T[] => {
		if (tracks.length < 2) return tracks;
		if (tracks[0].type === 'video') {
			if (codecs.length === 0) return mostEfficient(tracks);
			const rank = (track: T) => {
				const family = codecFamily(track.codec);
				const index = family ? codecs.indexOf(family) : -1;
				return index < 0 ? -codecs.length : -index;
			};
			return best(best(tracks, rank), (track) => track.representationCount ?? 0);
		}
		if (tracks[0].type === 'audio') return mostEfficient(selectAudio(tracks, audioLanguage));
		const prioritized = best(tracks, (track) => track.selectionPriority ?? 1);
		const main = prioritized.filter((track) => track.roles?.some((role) => role.value === 'main'));
		return main.length > 0 ? main : prioritized;
	};
}

/** Like the player's quality menu, a rendition's quality is its shorter side, so portrait works. */
export function renditionSize(rendition: RepresentationInfo): number | undefined {
	const width = Number(rendition.width) || 0;
	const height = Number(rendition.height) || 0;
	if (width && height) return Math.min(width, height);
	return height || width || undefined;
}

/** A dash.js capabilities filter dropping video renditions above `maxQuality`. */
export function qualityFilter(maxQuality: number) {
	return (representation: RepresentationInfo): boolean => {
		const size = renditionSize(representation);
		return size === undefined || size <= maxQuality;
	};
}

/** The best rendition at or below `target`, else the smallest one. */
export function pickRendition<T extends { id: string; width?: number; height?: number }>(
	renditions: readonly T[],
	target: number
): T | undefined {
	const sized = renditions
		.map((rendition) => ({ rendition, size: renditionSize(rendition) ?? 0 }))
		.sort((a, b) => b.size - a.size);
	return (sized.find((r) => r.size <= target) ?? sized.at(-1))?.rendition;
}

type DashEngine = {
	setCustomInitialTrackSelectionFunction(fn: (tracks: TrackInfo[]) => TrackInfo[]): void;
	resetCustomInitialTrackSelectionFunction(): void;
	registerCustomCapabilitiesFilter(filter: (rep: RepresentationInfo) => boolean): void;
	unregisterCustomCapabilitiesFilter(filter: (rep: RepresentationInfo) => boolean): void;
};

function isDashEngine(value: unknown): value is DashEngine {
	return (
		typeof value === 'object' &&
		value !== null &&
		typeof (value as DashEngine).setCustomInitialTrackSelectionFunction === 'function' &&
		typeof (value as DashEngine).registerCustomCapabilitiesFilter === 'function'
	);
}

/**
 * Hooks track preferences and quality limit into dash.js before it parses the manifest; returns
 * the cleanup. Does nothing without preferences, so dash.js keeps its own defaults.
 */
export function applyPlaybackPreferences(
	host: { readonly engine?: unknown },
	preferences: PlaybackPreferences | undefined
): () => void {
	const engine = host.engine;
	if (!preferences || !isDashEngine(engine)) return () => {};
	const cleanups: (() => void)[] = [];
	// Always set: dash.js would otherwise start with whatever track YouTube marks as default.
	engine.setCustomInitialTrackSelectionFunction(
		trackSelector(preferences.codecs, preferences.audioLanguage)
	);
	cleanups.push(() => engine.resetCustomInitialTrackSelectionFunction());
	if (preferences.maxQuality) {
		const filter = qualityFilter(preferences.maxQuality);
		engine.registerCustomCapabilitiesFilter(filter);
		cleanups.push(() => engine.unregisterCustomCapabilitiesFilter(filter));
	}
	return () => cleanups.forEach((cleanup) => cleanup());
}
