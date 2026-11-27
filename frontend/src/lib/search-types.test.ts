import { describe, expect, it } from 'vitest';
import { parseSearchFilters, parseSearchType, searchParams } from './search-types';

describe('search types', () => {
	it('parses known types and falls back to all', () => {
		expect(parseSearchType('shorts')).toBe('shorts');
		expect(parseSearchType('playlists')).toBe('playlists');
		expect(parseSearchType('movies')).toBe('all');
		expect(parseSearchType('Videos')).toBe('all');
		expect(parseSearchType(null)).toBe('all');
		expect(parseSearchType(undefined)).toBe('all');
	});

	it('builds the results query and leaves out the default type', () => {
		expect(searchParams(' cats ', 'all')).toBe('search_query=cats');
		expect(searchParams('cats & dogs', 'shorts')).toBe('search_query=cats+%26+dogs&type=shorts');
	});

	it('keeps only the filters the type offers', () => {
		const filters = { upload_date: 'week', duration: 'long', sort: 'views' } as const;
		expect(searchParams('cats', 'videos', filters)).toBe(
			'search_query=cats&type=videos&upload_date=week&duration=long&sort=views'
		);
		expect(searchParams('cats', 'shorts', filters)).toBe(
			'search_query=cats&type=shorts&upload_date=week&sort=views'
		);
		expect(searchParams('cats', 'channels', filters)).toBe('search_query=cats&type=channels');
	});

	it('parses known filters and drops unknown values and relevance', () => {
		const params = new URLSearchParams('upload_date=week&duration=epic&sort=relevance');
		expect(parseSearchFilters(params, 'all')).toEqual({ upload_date: 'week' });
		expect(parseSearchFilters(new URLSearchParams('sort=date&duration=short'), 'shorts')).toEqual({
			sort: 'date'
		});
		expect(parseSearchFilters(new URLSearchParams('sort=date'), 'playlists')).toEqual({});
	});
});
