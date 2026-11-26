import { createInfiniteQuery, type InfiniteData } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import type { VideoSummary } from '#lib/video.js';
import { api } from './client';
import { ApiError, pickImage, toVideo, uniqueBy, unwrap } from './mapping';
import { isLocalPlaylistId, isMixId } from './playlist-order';
import type { components } from './schema';

type Schemas = components['schemas'];

export type YouTubePlaylist = {
	id: string;
	title: string;
	description: string;
	channel?: { id: string; name: string };
	/** As reported by YouTube; hidden unavailable videos can make the list shorter. */
	videoCount: number;
	thumbnailUrl?: string;
	/** `YYYY-MM-DD` */
	lastUpdate?: string;
	lastUpdateText?: string;
};

/** The first page also carries the playlist itself. */
export type YouTubePlaylistPage = {
	playlist?: YouTubePlaylist;
	items: VideoSummary[];
	continuation?: string;
};

export type LoadedYouTubePlaylist = YouTubePlaylist & { items: VideoSummary[] };

export function toYouTubePlaylistPage(p: Schemas['YouTubePlaylist']): YouTubePlaylistPage {
	return {
		playlist: {
			id: p.id,
			title: p.title,
			description: p.description,
			channel: p.channel ? { id: p.channel.id, name: p.channel.name } : undefined,
			videoCount: p.video_count,
			thumbnailUrl: pickImage(p.thumbnails, 640),
			lastUpdate: p.last_update ?? undefined,
			lastUpdateText: p.last_update_text ?? undefined
		},
		items: p.videos.items.map((video) => toVideo(video)),
		continuation: p.videos.continuation ?? undefined
	};
}

/** All loaded pages as one list; a video listed twice keeps its first place. */
export function mergeYouTubePlaylist(
	data: InfiniteData<YouTubePlaylistPage> | undefined
): LoadedYouTubePlaylist | undefined {
	const playlist = data?.pages[0]?.playlist;
	if (!playlist) return undefined;
	const items = uniqueBy(
		data.pages.flatMap((page) => page.items),
		(video) => video.id
	);
	return { ...playlist, items };
}

/** Any ID that is neither ours nor a mix is looked up on YouTube. */
export function isYouTubePlaylistId(id: string | null | undefined): id is string {
	return typeof id === 'string' && id.length > 0 && !isLocalPlaylistId(id) && !isMixId(id);
}

export function createYouTubePlaylistQuery(id: () => string | null | undefined) {
	return createInfiniteQuery(() => {
		const listId = id() ?? '';
		const locale = getLocale();
		return {
			queryKey: ['youtube-playlist', locale, listId] as const,
			queryFn: async ({
				pageParam,
				signal
			}: {
				pageParam: string | undefined;
				signal: AbortSignal;
			}): Promise<YouTubePlaylistPage> => {
				const header = { 'Accept-Language': locale };
				if (!pageParam) {
					return toYouTubePlaylistPage(
						unwrap(
							await api.GET('/api/v0/youtube/playlists/{id}', {
								params: { path: { id: listId }, header },
								signal
							})
						)
					);
				}
				const page = unwrap(
					await api.GET('/api/v0/youtube/playlists/{id}/videos', {
						params: { path: { id: listId }, query: { continuation: pageParam }, header },
						signal
					})
				);
				return {
					items: page.items.map((video) => toVideo(video)),
					continuation: page.continuation ?? undefined
				};
			},
			initialPageParam: undefined as string | undefined,
			getNextPageParam: (last: YouTubePlaylistPage) => last.continuation,
			enabled: isYouTubePlaylistId(listId),
			staleTime: 5 * 60_000,
			// A missing, private or invalid playlist will not appear on retry.
			retry: (count: number, error: Error) =>
				!(error instanceof ApiError && error.status < 500) && count < 2
		};
	});
}
