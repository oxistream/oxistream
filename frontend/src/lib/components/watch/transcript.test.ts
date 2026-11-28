import { describe, expect, it } from 'vitest';
import {
	defaultTrack,
	findMatches,
	highlight,
	parseVtt,
	plainText,
	toTranscript
} from './transcript';

const MANUAL = [
	'\uFEFFWEBVTT',
	'Kind: captions',
	'Language: en',
	'',
	'STYLE',
	'::cue { color: yellow }',
	'',
	'NOTE written by hand',
	'',
	'intro',
	'00:00:01.000 --> 00:00:03.500 align:start position:10% line:90%',
	'<v Narrator>Welcome to the <b>forge</b>.</v>',
	'We start &amp; finish here.',
	'',
	'00:03.500 --> 00:05.000',
	'Tongs &lt;left&gt;&nbsp;hammer',
	'',
	'00:00:05.000 --> 00:00:06.000',
	'Tongs &lt;left&gt; hammer',
	''
].join('\r\n');

// The shape of YouTube's auto-generated captions: every cue repeats the line before, the new
// line carries karaoke timing, and 10 ms cues hold the finished line.
const AUTO = [
	'WEBVTT',
	'Kind: captions',
	'Language: en',
	'',
	'00:00:00.320 --> 00:00:02.390 align:start position:0%',
	' ',
	'so<00:00:00.640><c> today</c><00:00:00.960><c> we</c><00:00:01.200><c> forge</c>',
	'',
	'00:00:02.390 --> 00:00:02.400 align:start position:0%',
	'so today we forge',
	' ',
	'',
	'00:00:02.400 --> 00:00:04.830 align:start position:0%',
	'so today we forge',
	'a<00:00:02.720><c> bottle</c><00:00:03.040><c> opener</c>',
	'',
	'00:00:04.830 --> 00:00:04.840 align:start position:0%',
	'a bottle opener',
	' ',
	'',
	'00:00:04.840 --> 00:00:06.000 align:start position:0%',
	'a bottle opener',
	'yes',
	'',
	'00:00:06.000 --> 00:00:07.000 align:start position:0%',
	'yes',
	'yes<00:00:06.500><c> yes</c>',
	''
].join('\n');

describe('parseVtt', () => {
	it('reads cues with settings, identifiers and short timestamps, skipping other blocks', () => {
		expect(parseVtt(MANUAL)).toEqual([
			{
				start: 1,
				end: 3.5,
				text: '<v Narrator>Welcome to the <b>forge</b>.</v>\nWe start &amp; finish here.'
			},
			{ start: 3.5, end: 5, text: 'Tongs &lt;left&gt;&nbsp;hammer' },
			{ start: 5, end: 6, text: 'Tongs &lt;left&gt; hammer' }
		]);
	});

	it('drops the identifier of a cue that follows without a blank line', () => {
		const cues = parseVtt(
			'WEBVTT\n\n00:01.000 --> 00:02.000\none\n2\n00:02.000 --> 00:03.000\ntwo'
		);
		expect(cues.map((c) => c.text)).toEqual(['one', 'two']);
	});

	it('keeps a cue whose first line is blank', () => {
		expect(parseVtt('WEBVTT\n\n00:01.000 --> 00:02.000\n\nlate text\n')).toEqual([
			{ start: 1, end: 2, text: 'late text' }
		]);
	});

	it('reads hours and comma separators', () => {
		expect(parseVtt('WEBVTT\n\n1:02:03,5 --> 1:02:04,250\nx')).toEqual([
			{ start: 3723.5, end: 3724.25, text: 'x' }
		]);
	});

	it('ignores garbage', () => {
		expect(parseVtt('')).toEqual([]);
		expect(parseVtt('<html>not found</html>')).toEqual([]);
	});
});

describe('plainText', () => {
	it('strips tags and decodes entities', () => {
		expect(plainText('a<00:00:01.000><c.colorE5E5E5> b</c>  &#39;c&#x27; &amp;d &bogus;')).toBe(
			"a b 'c' &d &bogus;"
		);
	});
});

describe('toTranscript', () => {
	it('keeps only new lines of rolling auto-captions', () => {
		expect(toTranscript(parseVtt(AUTO))).toEqual([
			{ start: 0.32, end: 2.39, text: 'so today we forge' },
			{ start: 2.4, end: 4.83, text: 'a bottle opener' },
			{ start: 4.84, end: 6, text: 'yes' },
			{ start: 6, end: 7, text: 'yes yes' }
		]);
	});

	it('joins the lines of a cue and merges repeats', () => {
		expect(toTranscript(parseVtt(MANUAL))).toEqual([
			{ start: 1, end: 3.5, text: 'Welcome to the forge. We start & finish here.' },
			{ start: 3.5, end: 6, text: 'Tongs <left> hammer' }
		]);
	});
});

describe('search', () => {
	const lines = [
		{ start: 0, end: 1, text: 'Hammer time' },
		{ start: 1, end: 2, text: 'no match' },
		{ start: 2, end: 3, text: 'the hammer, the HAMMER' }
	];

	it('finds lines ignoring case', () => {
		expect(findMatches(lines, ' hammer ')).toEqual([0, 2]);
		expect(findMatches(lines, '  ')).toEqual([]);
	});

	it('splits text around matches', () => {
		expect(highlight(lines[2].text, 'hammer')).toEqual([
			{ text: 'the ', match: false },
			{ text: 'hammer', match: true },
			{ text: ', the ', match: false },
			{ text: 'HAMMER', match: true }
		]);
		expect(highlight('abc', '')).toEqual([{ text: 'abc', match: false }]);
	});
});

describe('defaultTrack', () => {
	const tracks = [
		{ id: 'de', lang: 'de', auto_generated: false },
		{ id: 'en-GB', lang: 'en-GB', auto_generated: false },
		{ id: 'a.en', lang: 'en', auto_generated: true }
	];

	it('prefers the captions showing in the player', () => {
		expect(defaultTrack(tracks, { showing: 'de' })?.id).toBe('de');
	});

	it('takes the original language, else the auto-generated track', () => {
		expect(defaultTrack(tracks)?.id).toBe('en-GB');
		expect(defaultTrack(tracks, { audioLang: 'de-DE' })?.id).toBe('de');
		expect(defaultTrack(tracks, { audioLang: 'fr' })?.id).toBe('a.en');
		expect(defaultTrack(tracks.slice(0, 1), { audioLang: 'fr' })?.id).toBe('de');
		expect(defaultTrack([])).toBeUndefined();
	});
});
