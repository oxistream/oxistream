import { describe, expect, it } from 'vitest';
import { isHandle, isVideoId, parseYouTubeUrl, resolvedPath, watchPath } from './youtube';

describe('isVideoId', () => {
	it('accepts real video ids', () => {
		for (const id of ['dQw4w9WgXcQ', 'J_IEMP-qlpU', 'cm94WV0ReKg', '8s5U6xeDDD4', '-_-_-_-_-_A']) {
			expect(isVideoId(id)).toBe(true);
		}
	});

	it('rejects other lengths, characters and impossible last characters', () => {
		for (const id of [
			'dQw4w9WgXc',
			'dQw4w9WgXcQQ',
			'dQw4w9WgX.Q',
			'information',
			'abcdefghijz',
			''
		]) {
			expect(isVideoId(id)).toBe(false);
		}
	});

	it('does not match app routes', () => {
		for (const route of ['feed', 'settings', 'login', 'register', 'watch', 'results', 'channel']) {
			expect(isVideoId(route)).toBe(false);
		}
	});
});

describe('isHandle', () => {
	it('accepts handles with a leading @', () => {
		expect(isHandle('@mkbhd')).toBe(true);
		expect(isHandle('@Linus.Tech-Tips_2')).toBe(true);
		expect(isHandle('@müller')).toBe(true);
	});

	it('rejects missing @, short, long and invalid handles', () => {
		expect(isHandle('mkbhd')).toBe(false);
		expect(isHandle('@ab')).toBe(false);
		expect(isHandle(`@${'a'.repeat(31)}`)).toBe(false);
		expect(isHandle('@with space')).toBe(false);
		expect(isHandle('@a/b/c')).toBe(false);
	});
});

describe('parseYouTubeUrl', () => {
	it('recognizes YouTube links with and without scheme', () => {
		for (const input of [
			'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
			'http://youtube.com/watch?v=dQw4w9WgXcQ&t=42',
			'youtu.be/dQw4w9WgXcQ',
			'm.youtube.com/@mkbhd',
			'music.youtube.com/playlist?list=PL123',
			'  https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ  ',
			'https://www.youtube.com/shorts/dQw4w9WgXcQ'
		]) {
			expect(parseYouTubeUrl(input), input).not.toBeNull();
		}
	});

	it('ignores search terms, other hosts and bare hosts', () => {
		for (const input of [
			'rust tutorial',
			'youtube',
			'youtube.com',
			'https://www.youtube.com/',
			'https://notyoutube.com/watch?v=dQw4w9WgXcQ',
			'https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ',
			'ftp://youtube.com/watch?v=x',
			'youtube.com/watch?v=x and more',
			''
		]) {
			expect(parseYouTubeUrl(input), input).toBeNull();
		}
	});
});

describe('resolvedPath', () => {
	it('maps videos, keeping the start offset', () => {
		expect(resolvedPath({ kind: 'video', id: 'dQw4w9WgXcQ' })).toBe('/watch?v=dQw4w9WgXcQ');
		expect(resolvedPath({ kind: 'video', id: 'dQw4w9WgXcQ', start_seconds: 42 })).toBe(
			'/watch?v=dQw4w9WgXcQ&t=42'
		);
		expect(resolvedPath({ kind: 'video', id: 'dQw4w9WgXcQ', start_seconds: 0 })).toBe(
			'/watch?v=dQw4w9WgXcQ'
		);
	});

	it('maps channels and playlists', () => {
		expect(resolvedPath({ kind: 'channel', id: 'UCBJycsmduvYEL83R_U4JriQ' })).toBe(
			'/channel/UCBJycsmduvYEL83R_U4JriQ'
		);
		expect(resolvedPath({ kind: 'playlist', id: 'PLx0sYbCqOb8TBPRdmBHs5Iftvv9TPboYG' })).toBe(
			'/playlist?list=PLx0sYbCqOb8TBPRdmBHs5Iftvv9TPboYG'
		);
	});
});

describe('watchPath', () => {
	it('builds a watch link and carries t over', () => {
		expect(watchPath('dQw4w9WgXcQ')).toBe('/watch?v=dQw4w9WgXcQ');
		expect(watchPath('dQw4w9WgXcQ', new URLSearchParams('t=42'))).toBe('/watch?v=dQw4w9WgXcQ&t=42');
		expect(watchPath('dQw4w9WgXcQ', new URLSearchParams('si=abc'))).toBe('/watch?v=dQw4w9WgXcQ');
	});
});
