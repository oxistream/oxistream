import { describe, expect, it } from 'vitest';
import { parseSearchType, searchParams } from './search-types';

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
});
