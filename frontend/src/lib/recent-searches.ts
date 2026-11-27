export const RECENT_SEARCHES_KEY = 'oxistream:recent-searches';
export const MAX_RECENT_SEARCHES = 10;

export function isRecentList(value: unknown): value is string[] {
	return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

function same(a: string, b: string) {
	return a.toLowerCase() === b.toLowerCase();
}

/** Puts `query` first, without case-insensitive duplicates, capped at the maximum. */
export function addRecent(list: readonly string[], query: string): string[] {
	const q = query.trim().replace(/\s+/g, ' ');
	if (!q) return [...list];
	return [q, ...list.filter((item) => !same(item, q))].slice(0, MAX_RECENT_SEARCHES);
}

export function removeRecent(list: readonly string[], query: string): string[] {
	return list.filter((item) => !same(item, query));
}

/** Recent searches that start with the typed text, minus an exact repeat of it. */
export function matchRecent(list: readonly string[], typed: string, limit: number): string[] {
	const t = typed.trim().toLowerCase();
	if (!t) return list.slice(0, limit);
	return list
		.filter((item) => {
			const lower = item.toLowerCase();
			return lower.startsWith(t) && lower !== t;
		})
		.slice(0, limit);
}
