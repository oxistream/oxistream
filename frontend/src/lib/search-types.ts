// Result type tabs of the search page, addressable via `/results?search_query=…&type=…`.

export const SEARCH_TYPES = ['all', 'videos', 'shorts', 'channels', 'playlists'] as const;
export type SearchType = (typeof SEARCH_TYPES)[number];

/** Unknown or missing types open the mixed results. */
export function parseSearchType(value: string | null | undefined): SearchType {
	return value && (SEARCH_TYPES as readonly string[]).includes(value)
		? (value as SearchType)
		: 'all';
}

/** Query string of a results page; `all` is left out. */
export function searchParams(query: string, type: SearchType): string {
	const params = new URLSearchParams({ search_query: query.trim() });
	if (type !== 'all') params.set('type', type);
	return params.toString();
}
