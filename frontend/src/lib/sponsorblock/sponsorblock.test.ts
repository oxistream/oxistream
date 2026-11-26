import { describe, expect, it } from 'vitest';
import { defaultSettings, normalizeSettings, type Segment } from './settings';
import { createSkipper, resolveSegments, segmentAt, type PlayerSegment } from './skip';

const segment = (
	id: string,
	category: Segment['category'],
	action: Segment['action'],
	start: number,
	end: number
): Segment => ({ id, category, action, start, end });

describe('normalizeSettings', () => {
	it('falls back to the defaults', () => {
		expect(normalizeSettings(null)).toEqual(defaultSettings());
		expect(normalizeSettings('nope')).toEqual(defaultSettings());
		expect(defaultSettings().categories.sponsor).toBe('auto');
		expect(defaultSettings().categories.intro).toBe('button');
	});

	it('keeps valid values only', () => {
		const settings = normalizeSettings({
			categories: { sponsor: 'off', intro: 'auto', outro: 'sometimes', unknown: 'auto' },
			highlight: false,
			full_video_label: 'yes'
		});
		expect(settings.categories.sponsor).toBe('off');
		expect(settings.categories.intro).toBe('auto');
		expect(settings.categories.outro).toBe('button');
		expect(settings.highlight).toBe(false);
		expect(settings.full_video_label).toBe(true);
	});
});

describe('resolveSegments', () => {
	const segments = [
		segment('s', 'sponsor', 'skip', 10, 20),
		segment('i', 'intro', 'skip', 0, 5),
		segment('f', 'filler', 'skip', 30, 40),
		segment('h', 'poi_highlight', 'poi', 50, 50),
		segment('x', 'exclusive_access', 'full', 0, 0)
	];

	it('applies the preferences', () => {
		const settings = defaultSettings();
		settings.categories.filler = 'off';
		expect(resolveSegments(segments, settings)).toEqual([
			{ id: 's', category: 'sponsor', kind: 'skip', mode: 'auto', start: 10, end: 20 },
			{ id: 'i', category: 'intro', kind: 'skip', mode: 'button', start: 0, end: 5 },
			{ id: 'h', category: 'poi_highlight', kind: 'highlight', start: 50, end: 50 },
			{ id: 'x', category: 'exclusive_access', kind: 'full', start: 0, end: 0 }
		]);
	});

	it('drops the highlight and label when turned off', () => {
		const settings = { ...defaultSettings(), highlight: false, full_video_label: false };
		expect(resolveSegments(segments, settings).map((s) => s.id)).toEqual(['s', 'i', 'f']);
	});
});

describe('createSkipper', () => {
	const auto = (id: string, start: number, end: number): PlayerSegment => ({
		id,
		category: 'sponsor',
		kind: 'skip',
		mode: 'auto',
		start,
		end
	});
	const button: PlayerSegment = { ...auto('b', 60, 70), category: 'intro', mode: 'button' };

	it('skips automatic segments to their end', () => {
		const skipper = createSkipper([auto('a', 10, 20), button]);
		expect(skipper.autoSkip(9.9)).toBeNull();
		expect(skipper.autoSkip(10)).toMatchObject({ segment: { id: 'a' }, target: 20 });
		expect(skipper.autoSkip(19.8)).toBeNull();
		expect(skipper.autoSkip(65)).toBeNull();
	});

	it('jumps over overlapping and adjacent automatic segments at once', () => {
		const skipper = createSkipper([auto('a', 10, 20), auto('b', 18, 30), auto('c', 30, 35)]);
		expect(skipper.autoSkip(12)?.target).toBe(35);
	});

	it('offers a button inside button segments', () => {
		const skipper = createSkipper([auto('a', 10, 20), button]);
		expect(skipper.button(15)).toBeUndefined();
		expect(skipper.button(62)?.id).toBe('b');
		expect(skipper.button(71)).toBeUndefined();
	});

	it('leaves segments alone that the user seeks into, but still offers to skip them', () => {
		const skipper = createSkipper([auto('a', 10, 20)]);
		skipper.seeked(12);
		expect(skipper.autoSkip(13)).toBeNull();
		expect(skipper.button(13)?.id).toBe('a');
	});

	it('does not skip again after an undo', () => {
		const skipper = createSkipper([auto('a', 10, 20)]);
		expect(skipper.autoSkip(10)).not.toBeNull();
		skipper.suppress('a');
		expect(skipper.autoSkip(10)).toBeNull();
	});

	it('ignores seeks outside automatic segments', () => {
		const skipper = createSkipper([auto('a', 10, 20)]);
		skipper.seeked(5);
		expect(skipper.autoSkip(10)).not.toBeNull();
	});
});

describe('upcoming skip', () => {
	const auto = (id: string, start: number, end: number): PlayerSegment => ({
		id,
		category: 'sponsor',
		kind: 'skip',
		mode: 'auto',
		start,
		end
	});

	it('looks ahead to the next automatic skip and its chained target', () => {
		const skipper = createSkipper([auto('b', 40, 50), auto('a', 10, 20), auto('c', 19, 25)]);
		expect(skipper.upcoming(5, 8)).toBe(25);
		expect(skipper.upcoming(0, 8)).toBeNull();
		expect(skipper.upcoming(12, 5)).toBeNull();
		expect(skipper.upcoming(12, 8)).toBe(25);
		expect(skipper.upcoming(35, 8)).toBe(50);
	});

	it('skips suppressed segments', () => {
		const skipper = createSkipper([auto('a', 10, 20)]);
		skipper.suppress('a');
		expect(skipper.upcoming(5, 8)).toBeNull();
	});
});

describe('segmentAt', () => {
	const segments: PlayerSegment[] = [
		{ id: 's', category: 'sponsor', kind: 'skip', mode: 'auto', start: 10, end: 30 },
		{ id: 'i', category: 'interaction', kind: 'skip', mode: 'button', start: 15, end: 18 },
		{ id: 'h', category: 'poi_highlight', kind: 'highlight', start: 50, end: 50 },
		{ id: 'x', category: 'exclusive_access', kind: 'full', start: 0, end: 0 }
	];

	it('finds the range under the pointer', () => {
		expect(segmentAt(segments, 10)?.id).toBe('s');
		expect(segmentAt(segments, 29.9)?.id).toBe('s');
		expect(segmentAt(segments, 30)).toBeUndefined();
		expect(segmentAt(segments, 5)).toBeUndefined();
	});

	it('prefers the shortest overlapping range', () => {
		expect(segmentAt(segments, 16)?.id).toBe('i');
	});

	it('finds a highlight near the pointer', () => {
		expect(segmentAt(segments, 50.4, 0.5)?.id).toBe('h');
		expect(segmentAt(segments, 51, 0.5)).toBeUndefined();
		expect(segmentAt(segments, 50)?.id).toBe('h');
	});

	it('ignores whole-video labels', () => {
		expect(segmentAt(segments, 0, 1)).toBeUndefined();
	});
});
