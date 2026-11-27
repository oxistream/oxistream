import type { TrendingCategory } from '#lib/api/trending.js';

export const HOME_SECTIONS = [
	'continue-watching',
	'subscriptions',
	'subscription-shorts',
	'trending-news',
	'trending-gaming',
	'trending-podcasts',
	'trending-music',
	'trending-sports',
	'trending-learning',
	'trending-fashion',
	'trending-live'
] as const;

export type HomeSectionKey = (typeof HOME_SECTIONS)[number];
export type Audience = 'user' | 'guest';

/** Sections that need an account; signed-in users may also choose the others. */
const AUDIENCE: Record<HomeSectionKey, Audience> = {
	'continue-watching': 'user',
	subscriptions: 'user',
	'subscription-shorts': 'user',
	'trending-news': 'guest',
	'trending-gaming': 'guest',
	'trending-podcasts': 'guest',
	'trending-music': 'guest',
	'trending-sports': 'guest',
	'trending-learning': 'guest',
	'trending-fashion': 'guest',
	'trending-live': 'guest'
};

export const DEFAULT_SECTIONS: Record<Audience, readonly HomeSectionKey[]> = {
	user: ['continue-watching', 'subscriptions', 'subscription-shorts'],
	guest: ['trending-news', 'trending-gaming', 'trending-podcasts']
};

function isSectionKey(value: string): value is HomeSectionKey {
	return (HOME_SECTIONS as readonly string[]).includes(value);
}

/**
 * Sections to render, in order. A user's own choice wins over the default; unknown keys and
 * duplicates are dropped. Guests always get their default, without account-only sections.
 */
export function homeSections(
	audience: Audience,
	preferred?: readonly string[] | null
): HomeSectionKey[] {
	const seen = new Set<HomeSectionKey>();
	const keys = audience === 'user' ? (preferred ?? DEFAULT_SECTIONS.user) : DEFAULT_SECTIONS.guest;
	return keys.filter((value): value is HomeSectionKey => {
		if (!isSectionKey(value) || seen.has(value)) return false;
		if (audience === 'guest' && AUDIENCE[value] === 'user') return false;
		seen.add(value);
		return true;
	});
}

export type EditableSection = { key: HomeSectionKey; enabled: boolean };

/** Every section for the settings: the enabled ones in their order, then the rest, disabled. */
export function editableSections(preferred?: readonly string[] | null): EditableSection[] {
	const enabled = homeSections('user', preferred);
	return [
		...enabled.map((key) => ({ key, enabled: true })),
		...HOME_SECTIONS.filter((key) => !enabled.includes(key)).map((key) => ({
			key,
			enabled: false
		}))
	];
}

/** What gets stored: the enabled keys in order. */
export function enabledSections(sections: readonly EditableSection[]): HomeSectionKey[] {
	return sections.filter((section) => section.enabled).map((section) => section.key);
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
