export type VideoCodec = 'av1' | 'vp9' | 'h264';

/** Codec and quality preferences from the user's settings. */
export type PlaybackPreferences = {
	/** Most preferred first; empty leaves the choice to dash.js. */
	codecs: readonly VideoCodec[];
	/** Highest rendition, by its shorter side; `null` for no limit. */
	maxQuality: number | null;
	/** Rendition to start with instead of adaptive quality. */
	defaultQuality: number | null;
};

/** The parts of a dash.js `MediaInfo` the initial track selection reads. */
export type TrackInfo = {
	type?: string | null;
	codec?: string | null;
	selectionPriority?: number;
	representationCount?: number;
	roles?: { value: string }[] | null;
};

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

/**
 * Replaces dash.js's initial track selection: video tracks by codec preference, the widest
 * range first among equals; other tracks as dash.js does by default, by `selectionPriority`
 * and then the main role.
 */
export function trackSelector(codecs: readonly VideoCodec[]) {
	return <T extends TrackInfo>(tracks: T[]): T[] => {
		if (tracks.length < 2) return tracks;
		if (tracks[0].type === 'video') {
			const rank = (track: T) => {
				const family = codecFamily(track.codec);
				const index = family ? codecs.indexOf(family) : -1;
				return index < 0 ? -codecs.length : -index;
			};
			return best(best(tracks, rank), (track) => track.representationCount ?? 0);
		}
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
 * Hooks codec preference and quality limit into dash.js before it parses the manifest; returns
 * the cleanup. Does nothing without preferences, so dash.js keeps its own defaults.
 */
export function applyPlaybackPreferences(
	host: { readonly engine?: unknown },
	preferences: PlaybackPreferences | undefined
): () => void {
	const engine = host.engine;
	if (!preferences || !isDashEngine(engine)) return () => {};
	const cleanups: (() => void)[] = [];
	if (preferences.codecs.length > 0) {
		engine.setCustomInitialTrackSelectionFunction(trackSelector(preferences.codecs));
		cleanups.push(() => engine.resetCustomInitialTrackSelectionFunction());
	}
	if (preferences.maxQuality) {
		const filter = qualityFilter(preferences.maxQuality);
		engine.registerCustomCapabilitiesFilter(filter);
		cleanups.push(() => engine.unregisterCustomCapabilitiesFilter(filter));
	}
	return () => cleanups.forEach((cleanup) => cleanup());
}
