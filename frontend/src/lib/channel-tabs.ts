// Channel tabs and their URLs, which mirror YouTube's (`/@handle/shorts`, `/channel/UC…/streams`).

export const CHANNEL_TABS = ['videos', 'shorts', 'streams', 'playlists', 'search'] as const;
export type ChannelTab = (typeof CHANNEL_TABS)[number];
/** Tabs that list videos and can be sorted. */
export type VideoTab = 'videos' | 'shorts' | 'streams';

export const CHANNEL_SORTS = ['latest', 'popular', 'oldest'] as const;
export type ChannelSort = (typeof CHANNEL_SORTS)[number];

export function isChannelTab(value: string): value is ChannelTab {
	return (CHANNEL_TABS as readonly string[]).includes(value);
}

export function isVideoTab(tab: ChannelTab): tab is VideoTab {
	return tab === 'videos' || tab === 'shorts' || tab === 'streams';
}

/** The tab of an optional route segment; a bare channel URL opens the videos. */
export function parseChannelTab(value: string | undefined): ChannelTab {
	return value && isChannelTab(value) ? value : 'videos';
}

export function parseChannelSort(value: string | null | undefined): ChannelSort {
	return value && (CHANNEL_SORTS as readonly string[]).includes(value)
		? (value as ChannelSort)
		: 'latest';
}

/**
 * The URL of a channel tab below `base` (`/@handle` or `/channel/UC…`). The default sort and an
 * empty query are left out.
 */
export function channelTabHref(
	base: string,
	tab: ChannelTab,
	options: { sort?: ChannelSort; query?: string } = {}
): string {
	const params = new URLSearchParams();
	if (tab === 'search') {
		const query = options.query?.trim();
		if (query) params.set('query', query);
	} else if (isVideoTab(tab) && options.sort && options.sort !== 'latest') {
		params.set('sort', options.sort);
	}
	const search = params.toString();
	return `${base}/${tab}${search ? `?${search}` : ''}`;
}

/**
 * Tabs shown in the tab bar. Shorts and live are hidden when YouTube says the channel has none,
 * unless one of them was opened directly. Search lives in its own field.
 */
export function visibleTabs(
	channel: { hasShorts: boolean; hasLive: boolean },
	current: ChannelTab
): Exclude<ChannelTab, 'search'>[] {
	return (['videos', 'shorts', 'streams', 'playlists'] as const).filter((tab) => {
		if (tab === current) return true;
		if (tab === 'shorts') return channel.hasShorts;
		if (tab === 'streams') return channel.hasLive;
		return true;
	});
}

/** API path segment of a video tab. */
export function apiTabSegment(tab: VideoTab): 'videos' | 'shorts' | 'live' {
	return tab === 'streams' ? 'live' : tab;
}

/** The thumbnail of a short: the tallest portrait image, else the tallest one. */
export function pickPortrait<T extends { width: number; height: number }>(
	images: T[] | undefined
): T | undefined {
	if (!images?.length) return undefined;
	const portrait = images.filter((image) => image.height > image.width);
	const pool = portrait.length ? portrait : images;
	return pool.reduce((best, image) => {
		const ratio = image.height / image.width;
		const bestRatio = best.height / best.width;
		// Rounded sizes make equal shapes differ slightly.
		if (Math.abs(ratio - bestRatio) > 0.02) return ratio > bestRatio ? image : best;
		return image.height > best.height ? image : best;
	});
}
