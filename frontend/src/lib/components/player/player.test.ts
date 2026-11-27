import { describe, expect, it, vi } from 'vitest';
import { groupAudioTracks, pickTrack } from './dash-audio-tracks';
import { createProgressEmitter, type ProgressReport } from './progress';
import { parseStartTime } from './start-time';

describe('parseStartTime', () => {
	it.each([
		['90', 90],
		['90s', 90],
		['1m30s', 90],
		['1h2m3s', 3723],
		['2m', 120],
		['1:30', 90],
		['01:02:03', 3723],
		['12.7', 12],
		[' 45 ', 45]
	])('%s → %d', (input, expected) => {
		expect(parseStartTime(input)).toBe(expected);
	});

	it.each([null, undefined, '', 'abc', '1x', 'h', '1:2:3:4'])('rejects %s', (input) => {
		expect(parseStartTime(input)).toBeNull();
	});
});

describe('createProgressEmitter', () => {
	function setup() {
		let time = 0;
		const reports: ProgressReport[] = [];
		const emitter = createProgressEmitter((r) => reports.push(r), { now: () => time });
		return { emitter, reports, advance: (ms: number) => (time += ms) };
	}

	it('reports every ten seconds while playing', () => {
		const { emitter, reports, advance } = setup();
		emitter.update(0, 100, true);
		advance(5_000);
		emitter.update(5, 100, true);
		expect(reports).toHaveLength(0);
		advance(5_000);
		emitter.update(10, 100, true);
		advance(10_000);
		emitter.update(20, 100, true);
		expect(reports.map((r) => [r.position, r.reason])).toEqual([
			[10, 'interval'],
			[20, 'interval']
		]);
	});

	it('stays quiet while paused', () => {
		const { emitter, reports, advance } = setup();
		emitter.update(0, 100, true);
		advance(60_000);
		emitter.update(3, 100, false);
		expect(reports).toHaveLength(0);
	});

	it('reports on pause and unload but not the same position twice', () => {
		const { emitter, reports } = setup();
		emitter.flush(42.04, 100, 'pause');
		emitter.flush(42.04, 100, 'unload');
		emitter.flush(43, 100, 'unload');
		expect(reports.map((r) => [r.position, r.reason])).toEqual([
			[42, 'pause'],
			[43, 'unload']
		]);
	});

	it('counts 90 % or the end as finished', () => {
		const { emitter, reports } = setup();
		emitter.flush(89, 100, 'pause');
		emitter.flush(90, 100, 'pause');
		emitter.flush(10, 100, 'ended');
		expect(reports.map((r) => r.finished)).toEqual([false, true, true]);
	});

	it('ignores reports without a known duration', () => {
		const onprogress = vi.fn();
		const emitter = createProgressEmitter(onprogress);
		emitter.flush(5, 0, 'pause');
		emitter.flush(5, Number.NaN, 'pause');
		expect(onprogress).not.toHaveBeenCalled();
	});
});

describe('groupAudioTracks', () => {
	const track = (id: string, lang: string, label: string, mimeType: string, role = 'dub') => ({
		id,
		lang,
		labels: [{ text: label }],
		roles: [{ value: role }],
		mimeType
	});
	const tracks = [
		track('3', 'en', 'English original', 'audio/mp4', 'main'),
		track('4', 'en', 'English original', 'audio/webm', 'main'),
		track('5', 'de', 'German', 'audio/mp4'),
		track('6', 'de', 'German', 'audio/webm')
	];

	it('merges codec variants of one language', () => {
		const groups = groupAudioTracks(tracks);
		expect(groups.map((g) => [g.label, g.language, g.main, g.tracks.length])).toEqual([
			['English original', 'en', true, 2],
			['German', 'de', false, 2]
		]);
	});

	it('keeps the codec that is playing when switching language', () => {
		const [, german] = groupAudioTracks(tracks);
		expect(pickTrack(german, tracks[1]).id).toBe('6');
		expect(pickTrack(german, null).id).toBe('5');
	});

	it('marks auto-dubbed languages', () => {
		const auto = {
			...track('7', 'fr', 'French', 'audio/mp4'),
			roles: [{ value: 'dub' }, { schemeIdUri: 'urn:oxistream:dash:role:2026', value: 'auto-dub' }]
		};
		const [french] = groupAudioTracks([auto]);
		expect(french.label).toBe('French (auto-dubbed)');
		expect(french.main).toBe(false);
	});
});
