import { describe, expect, it, vi } from 'vitest';
import {
	applyPlaybackPreferences,
	audioKind,
	codecFamily,
	pickRendition,
	qualityFilter,
	selectAudio,
	trackSelector
} from './playback';

describe('codec family', () => {
	it('reads bare and mime-wrapped codec strings', () => {
		expect(codecFamily('av01.0.08M.08')).toBe('av1');
		expect(codecFamily('video/webm;codecs="vp09.00.51.08"')).toBe('vp9');
		expect(codecFamily('vp9')).toBe('vp9');
		expect(codecFamily('video/mp4;codecs="avc1.64001F"')).toBe('h264');
		expect(codecFamily('mp4a.40.2')).toBeUndefined();
		expect(codecFamily(null)).toBeUndefined();
	});
});

describe('track selection', () => {
	const video = (codec: string, representationCount = 5) => ({
		type: 'video',
		codec,
		representationCount
	});

	it('prefers video tracks by codec order, then the widest range', () => {
		const av1 = video('av01.0.08M.08');
		const vp9Sdr = video('vp09.00.51.08', 8);
		const vp9Hdr = video('vp09.02.51.10', 4);
		const avc = video('avc1.64001F');
		const select = trackSelector(['vp9', 'av1']);
		expect(select([av1, avc, vp9Hdr, vp9Sdr])).toEqual([vp9Sdr]);
		expect(trackSelector(['h264'])([av1, avc])).toEqual([avc]);
		expect(trackSelector(['av1'])([avc, vp9Sdr]), 'unlisted codecs tie').toEqual([vp9Sdr]);
	});

	it('picks the most efficient video track without a codec order', () => {
		const avc = {
			...video('avc1.64001F'),
			bitrateList: [{ bandwidth: 4000, width: 10, height: 10 }]
		};
		const vp9 = {
			...video('vp09.00.51.08'),
			bitrateList: [{ bandwidth: 2000, width: 10, height: 10 }]
		};
		expect(trackSelector([])([avc, vp9])).toEqual([vp9]);
	});

	const AUTO_DUB = { schemeIdUri: 'urn:oxistream:dash:role:2026', value: 'auto-dub' };
	const audio = (lang: string, role: string, auto = false, bandwidth = 128) => ({
		type: 'audio',
		lang,
		selectionPriority: role === 'main' ? 2 : 1,
		roles: [{ schemeIdUri: 'urn:mpeg:dash:role:2011', value: role }, ...(auto ? [AUTO_DUB] : [])],
		bitrateList: [{ bandwidth }]
	});

	it('classifies audio tracks by their roles', () => {
		expect(audioKind(audio('en', 'main').roles)).toBe('original');
		expect(audioKind(audio('de', 'dub').roles)).toBe('dubbed');
		expect(audioKind(audio('de', 'dub', true).roles)).toBe('auto-dubbed');
		expect(audioKind([{ value: 'main' }]), 'scheme defaults to DASH').toBe('original');
		expect(audioKind(null)).toBe('other');
	});

	it('starts with the original audio unless a language is preferred', () => {
		const original = audio('en-US', 'main');
		const originalOpus = audio('en-US', 'main', false, 120);
		const dubbed = audio('fr', 'dub');
		const autoDe = audio('de-DE', 'dub', true);
		const tracks = [autoDe, dubbed, original, originalOpus];
		expect(selectAudio(tracks, null)).toEqual([original, originalOpus]);
		expect(trackSelector([])(tracks), 'most efficient codec').toEqual([originalOpus]);
		expect(selectAudio(tracks, 'fr'), 'human dub').toEqual([dubbed]);
		expect(selectAudio(tracks, 'de'), 'never an auto-dub over the original').toEqual([
			original,
			originalOpus
		]);
		expect(selectAudio(tracks, 'en'), 'original in the language').toEqual([original, originalOpus]);
	});

	it('prefers the exact region, and falls back without an original', () => {
		const pt = audio('pt', 'dub');
		const ptBr = audio('pt-BR', 'dub');
		const autoDe = audio('de', 'dub', true);
		const autoEs = audio('es', 'dub', true);
		expect(selectAudio([pt, ptBr], 'pt-BR')).toEqual([ptBr]);
		expect(selectAudio([pt, ptBr], 'pt-PT')).toEqual([pt, ptBr]);
		expect(selectAudio([autoEs, pt], 'de'), 'any track that is not auto-dubbed').toEqual([pt]);
		expect(selectAudio([autoEs, autoDe], 'de'), 'only auto-dubs').toEqual([autoDe]);
	});
});

describe('quality', () => {
	it('filters renditions by their shorter side and keeps audio', () => {
		const filter = qualityFilter(1080);
		expect(filter({ width: 1920, height: 1080 })).toBe(true);
		expect(filter({ width: 2560, height: 1440 })).toBe(false);
		expect(filter({ width: '1080', height: '1920' }), 'portrait 1080p').toBe(true);
		expect(filter({}), 'audio has no size').toBe(true);
	});

	it('picks the best rendition at or below the target', () => {
		const renditions = [
			{ id: 'a', width: 640, height: 360 },
			{ id: 'b', width: 1920, height: 1080 },
			{ id: 'c', width: 1280, height: 720 }
		];
		expect(pickRendition(renditions, 1080)?.id).toBe('b');
		expect(pickRendition(renditions, 1000)?.id).toBe('c');
		expect(pickRendition(renditions, 144)?.id, 'else the smallest').toBe('a');
		expect(pickRendition([], 720)).toBeUndefined();
	});
});

describe('engine hooks', () => {
	const engine = () => ({
		setCustomInitialTrackSelectionFunction: vi.fn(),
		resetCustomInitialTrackSelectionFunction: vi.fn(),
		registerCustomCapabilitiesFilter: vi.fn(),
		unregisterCustomCapabilitiesFilter: vi.fn()
	});

	it('registers only what is set and cleans up', () => {
		const e = engine();
		const cleanup = applyPlaybackPreferences(
			{ engine: e },
			{ codecs: ['av1'], maxQuality: 720, defaultQuality: null, audioLanguage: null }
		);
		expect(e.setCustomInitialTrackSelectionFunction).toHaveBeenCalledOnce();
		expect(e.registerCustomCapabilitiesFilter).toHaveBeenCalledOnce();
		cleanup();
		expect(e.resetCustomInitialTrackSelectionFunction).toHaveBeenCalledOnce();
		expect(e.unregisterCustomCapabilitiesFilter).toHaveBeenCalledWith(
			e.registerCustomCapabilitiesFilter.mock.calls[0][0]
		);

		const idle = engine();
		applyPlaybackPreferences(
			{ engine: idle },
			{ codecs: [], maxQuality: null, defaultQuality: 720, audioLanguage: null }
		)();
		expect(idle.setCustomInitialTrackSelectionFunction, 'audio always').toHaveBeenCalledOnce();
		expect(idle.registerCustomCapabilitiesFilter).not.toHaveBeenCalled();
		expect(() => applyPlaybackPreferences({}, undefined)()).not.toThrow();
	});
});
