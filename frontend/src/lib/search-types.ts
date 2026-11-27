// Result type tabs and filters of the search page, addressable via
// `/results?search_query=…&type=…&upload_date=…&duration=…&sort=…`.

export const SEARCH_TYPES = ['all', 'videos', 'shorts', 'channels', 'playlists'] as const;
export type SearchType = (typeof SEARCH_TYPES)[number];

export const UPLOAD_DATES = ['hour', 'day', 'week', 'month', 'year'] as const;
export type UploadDate = (typeof UPLOAD_DATES)[number];

export const DURATIONS = ['short', 'medium', 'long'] as const;
export type Duration = (typeof DURATIONS)[number];

export const SEARCH_SORTS = ['relevance', 'date', 'views'] as const;
export type SearchSort = (typeof SEARCH_SORTS)[number];

export type SearchFilters = {
	upload_date?: UploadDate;
	duration?: Duration;
	/** Missing means relevance. */
	sort?: Exclude<SearchSort, 'relevance'>;
};
export type SearchFilter = keyof SearchFilters;

const FILTERS: Record<SearchType, readonly SearchFilter[]> = {
	all: ['upload_date', 'duration', 'sort'],
	videos: ['upload_date', 'duration', 'sort'],
	shorts: ['upload_date', 'sort'],
	channels: [],
	playlists: []
};

/** Filters offered on a result type's tab. */
export function filtersFor(type: SearchType): readonly SearchFilter[] {
	return FILTERS[type];
}

/** Unknown or missing types open the mixed results. */
export function parseSearchType(value: string | null | undefined): SearchType {
	return value && (SEARCH_TYPES as readonly string[]).includes(value)
		? (value as SearchType)
		: 'all';
}

function pick<T extends string>(values: readonly T[], value: string | null): T | undefined {
	return value && (values as readonly string[]).includes(value) ? (value as T) : undefined;
}

/** Known filters from a results URL that apply to the type; the rest is ignored. */
export function parseSearchFilters(
	params: Pick<URLSearchParams, 'get'>,
	type: SearchType
): SearchFilters {
	const sort = pick(SEARCH_SORTS, params.get('sort'));
	return onlyFor(type, {
		upload_date: pick(UPLOAD_DATES, params.get('upload_date')),
		duration: pick(DURATIONS, params.get('duration')),
		sort: sort === 'relevance' ? undefined : sort
	});
}

function onlyFor(type: SearchType, filters: SearchFilters): SearchFilters {
	return Object.fromEntries(
		filtersFor(type).flatMap((key) => (filters[key] ? [[key, filters[key]]] : []))
	);
}

/** Query string of a results page; `all` and filters the type does not offer are left out. */
export function searchParams(query: string, type: SearchType, filters: SearchFilters = {}): string {
	const params = new URLSearchParams({ search_query: query.trim() });
	if (type !== 'all') params.set('type', type);
	for (const [key, value] of Object.entries(onlyFor(type, filters))) params.set(key, value);
	return params.toString();
}
