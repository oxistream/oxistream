import { describe, expect, it } from 'vitest';
import { parseDescription, parseTimestamp, toLocalPath } from './description';

describe('parseTimestamp', () => {
	it('reads m:ss and h:mm:ss', () => {
		expect(parseTimestamp('0:05')).toBe(5);
		expect(parseTimestamp('4:05')).toBe(245);
		expect(parseTimestamp('1:02:03')).toBe(3723);
	});
});

describe('parseDescription', () => {
	it('keeps plain text as one segment', () => {
		expect(parseDescription('Just words.\nTwo lines.')).toEqual([
			{ type: 'text', text: 'Just words.\nTwo lines.' }
		]);
	});

	it('links URLs and leaves trailing punctuation outside', () => {
		expect(parseDescription('See https://example.com/a?b=1. Thanks')).toEqual([
			{ type: 'text', text: 'See ' },
			{
				type: 'link',
				text: 'https://example.com/a?b=1',
				href: 'https://example.com/a?b=1',
				internal: false
			},
			{ type: 'text', text: '. Thanks' }
		]);
	});

	it('keeps brackets that belong to the URL', () => {
		const [, link, rest] = parseDescription('(https://en.wikipedia.org/wiki/Rust_(language))');
		expect(link).toMatchObject({ text: 'https://en.wikipedia.org/wiki/Rust_(language)' });
		expect(rest).toEqual({ type: 'text', text: ')' });
	});

	it('turns chapter timestamps into seek targets', () => {
		expect(parseDescription('0:00 Intro\n12:34 Build\n1:02:03 Outro')).toEqual([
			{ type: 'timestamp', text: '0:00', seconds: 0 },
			{ type: 'text', text: ' Intro\n' },
			{ type: 'timestamp', text: '12:34', seconds: 754 },
			{ type: 'text', text: ' Build\n' },
			{ type: 'timestamp', text: '1:02:03', seconds: 3723 },
			{ type: 'text', text: ' Outro' }
		]);
	});

	it('ignores timestamps past the end and things that only look like one', () => {
		const segments = parseDescription('At 9:59 and 10:01, ratio 16:9, time 12:345, v1.2:30', 600);
		expect(segments.filter((s) => s.type === 'timestamp')).toEqual([
			{ type: 'timestamp', text: '9:59', seconds: 599 }
		]);
	});

	it('rewrites YouTube links to local routes', () => {
		const [link] = parseDescription('https://www.youtube.com/watch?v=q6fgVVmdpgY');
		expect(link).toEqual({
			type: 'link',
			text: 'https://www.youtube.com/watch?v=q6fgVVmdpgY',
			href: '/watch?v=q6fgVVmdpgY',
			internal: true
		});
	});
});

describe('toLocalPath', () => {
	it.each([
		['https://youtu.be/dQw4w9WgXcQ?t=42', '/watch?v=dQw4w9WgXcQ&t=42'],
		['https://m.youtube.com/watch?v=dQw4w9WgXcQ&t=1m2s', '/watch?v=dQw4w9WgXcQ&t=1m2s'],
		['https://www.youtube.com/shorts/dQw4w9WgXcQ', '/watch?v=dQw4w9WgXcQ'],
		[
			'https://www.youtube.com/channel/UC4-79UOlP48-QNGgCko5p2g',
			'/channel/UC4-79UOlP48-QNGgCko5p2g'
		],
		['https://www.youtube.com/@mrbeast/videos', '/@mrbeast'],
		['https://www.youtube.com/playlist?list=PL123', '/playlist?list=PL123'],
		['https://www.youtube.com/results?search_query=a+b', '/results?search_query=a%20b']
	])('%s → %s', (input, expected) => {
		expect(toLocalPath(input)).toBe(expected);
	});

	it.each([
		'https://example.com/watch?v=dQw4w9WgXcQ',
		'https://www.youtube.com/about',
		'https://youtu.be/short',
		'not a url'
	])('leaves %s alone', (input) => {
		expect(toLocalPath(input)).toBeNull();
	});
});
