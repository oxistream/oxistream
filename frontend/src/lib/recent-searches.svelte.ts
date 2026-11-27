import { readJson, writeJson } from '#lib/components/player/storage.js';
import {
	RECENT_SEARCHES_KEY,
	addRecent,
	isRecentList,
	removeRecent
} from '#lib/recent-searches.js';

/** Searches made in this browser, newest first. Kept in localStorage only. */
class RecentSearches {
	items = $state<string[]>([]);

	constructor() {
		if (typeof window === 'undefined') return;
		this.load();
		window.addEventListener('storage', (event) => {
			if (event.key === RECENT_SEARCHES_KEY || event.key === null) this.load();
		});
	}

	private load() {
		this.items = readJson(RECENT_SEARCHES_KEY, isRecentList) ?? [];
	}

	private save(items: string[]) {
		this.items = items;
		writeJson(RECENT_SEARCHES_KEY, items);
	}

	add(query: string) {
		this.save(addRecent(this.items, query));
	}

	remove(query: string) {
		this.save(removeRecent(this.items, query));
	}

	clear() {
		this.save([]);
	}
}

export const recentSearches = new RecentSearches();
