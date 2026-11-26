import { resolve } from '$app/paths';
import type { ResolvedPathname } from '$app/types';

export function playlistWatchHref(videoId: string, listId: string): ResolvedPathname {
	const query = new URLSearchParams({ v: videoId, list: listId });
	return `${resolve('/watch')}?${query}` as ResolvedPathname;
}

export function playlistHref(listId: string): ResolvedPathname {
	return `${resolve('/playlist')}?${new URLSearchParams({ list: listId })}` as ResolvedPathname;
}
