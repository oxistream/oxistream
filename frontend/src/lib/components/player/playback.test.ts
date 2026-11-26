import { describe, expect, it, vi } from 'vitest';
import {
	applyPlaybackPreferences,
	codecFamily,
	pickRendition,
	qualityFilter,
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

	it('keeps the default track for audio', () => {
		const dubbed = { type: 'audio', selectionPriority: 1, roles: [{ value: 'dub' }] };
		const original = { type: 'audio', selectionPriority: 2, roles: [{ value: 'main' }] };
		const originalOpus = { ...original, codec: 'opus' };
		expect(trackSelector(['av1'])([dubbed, original, originalOpus])).toEqual([
			original,
			originalOpus
		]);
		expect(trackSelector(['av1'])([dubbed])).toEqual([dubbed]);
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
			{ codecs: ['av1'], maxQuality: 720, defaultQuality: null }
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
			{ codecs: [], maxQuality: null, defaultQuality: 720 }
		)();
		expect(idle.setCustomInitialTrackSelectionFunction).not.toHaveBeenCalled();
		expect(idle.registerCustomCapabilitiesFilter).not.toHaveBeenCalled();
		expect(() => applyPlaybackPreferences({}, undefined)()).not.toThrow();
	});
});
