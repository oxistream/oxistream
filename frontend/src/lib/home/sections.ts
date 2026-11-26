import type { TrendingCategory } from '#lib/api/trending.js';

export const HOME_SECTIONS = [
	'continue-watching',
	'subscriptions',
	'subscription-shorts',
	'trending-news',
	'trending-gaming',
	'trending-podcasts'
] as const;

export type HomeSectionKey = (typeof HOME_SECTIONS)[number];
export type Audience = 'user' | 'guest';

const AUDIENCE: Record<HomeSectionKey, Audience> = {
	'continue-watching': 'user',
	subscriptions: 'user',
	'subscription-shorts': 'user',
	'trending-news': 'guest',
	'trending-gaming': 'guest',
	'trending-podcasts': 'guest'
};

export const DEFAULT_SECTIONS: Record<Audience, readonly HomeSectionKey[]> = {
	user: ['continue-watching', 'subscriptions', 'subscription-shorts'],
	guest: ['trending-news', 'trending-gaming', 'trending-podcasts']
};

function isSectionKey(value: string): value is HomeSectionKey {
	return (HOME_SECTIONS as readonly string[]).includes(value);
}

/**
 * Sections to render, in order. A user's own choice wins over the default; unknown keys,
 * duplicates and sections of the other audience are dropped.
 */
export function homeSections(audience: Audience, preferred?: readonly string[]): HomeSectionKey[] {
	const seen = new Set<HomeSectionKey>();
	return (preferred ?? DEFAULT_SECTIONS[audience]).filter((value): value is HomeSectionKey => {
		if (!isSectionKey(value) || AUDIENCE[value] !== audience || seen.has(value)) return false;
		seen.add(value);
		return true;
	});
}

export function trendingCategory(key: HomeSectionKey): TrendingCategory | undefined {
	return key.startsWith('trending-')
		? (key.slice('trending-'.length) as TrendingCategory)
		: undefined;
}

export type RowState = 'loading' | 'shown' | 'hidden';

/** Rows that come and go with their data: hidden once they turn out empty or fail. */
export function rowState(query: { isPending: boolean; count: number }): RowState {
	if (query.count > 0) return 'shown';
	return query.isPending ? 'loading' : 'hidden';
}

/** Guests get a search box and their local history once no trending row has anything to show. */
export function showGuestFallback(trending: { pending: boolean; withVideos: number }): boolean {
	return !trending.pending && trending.withVideos === 0;
}
