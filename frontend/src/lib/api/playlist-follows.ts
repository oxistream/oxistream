import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import { session } from '#lib/session.svelte.js';
import { api } from './client';
import { pickImage, unwrap } from './mapping';
import { playlistKeys, toPlaylist, type Playlist } from './playlists';
import type { components } from './schema';

export type FollowedPlaylist = {
	id: string;
	/** Missing until the feed worker fetched the playlist once. */
	title?: string;
	channel?: { id: string; name: string };
	thumbnailUrl?: string;
	videoCount?: number;
	/** Videos added since the user last opened the playlist. */
	newVideoCount: number;
	/** The last fetch failed: deleted, made private, or YouTube was unreachable. */
	unavailable: boolean;
	followedAt: string;
};

// While a new playlist waits for its first fetch, the list is polled until it lands.
const PENDING_POLL_MS = 4000;

export function toFollowedPlaylist(p: components['schemas']['FollowedPlaylist']): FollowedPlaylist {
	return {
		id: p.id,
		title: p.title?.trim() || undefined,
		channel: p.channel ? { id: p.channel.id, name: p.channel.name } : undefined,
		thumbnailUrl: pickImage(p.thumbnails, 320),
		videoCount: p.video_count ?? undefined,
		newVideoCount: p.new_video_count,
		unavailable: p.unavailable,
		followedAt: p.followed_at
	};
}

/** Pending playlists have neither a title nor a failed fetch yet. */
export function hasPendingPlaylists(list: readonly FollowedPlaylist[]): boolean {
	return list.some((p) => p.title === undefined && !p.unavailable);
}

export const followKeys = {
	all: ['playlist-follows'] as const,
	list: ['playlist-follows', 'list'] as const,
	state: (id: string) => ['playlist-follows', 'state', id] as const
};

export function createFollowedPlaylistsQuery() {
	return createQuery(() => ({
		queryKey: followKeys.list,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/playlist-follows', { signal })).map(toFollowedPlaylist),
		enabled: session.user !== null,
		staleTime: 60_000,
		refetchInterval: (query: { state: { data?: FollowedPlaylist[] } }) =>
			query.state.data && hasPendingPlaylists(query.state.data) ? PENDING_POLL_MS : false
	}));
}

export function createFollowStateQuery(playlistId: () => string | undefined) {
	return createQuery(() => {
		const id = playlistId() ?? '';
		return {
			queryKey: followKeys.state(id),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/playlist-follows/{playlist_id}', {
						params: { path: { playlist_id: id } },
						signal
					})
				).following,
			enabled: session.user !== null && id.length > 0,
			staleTime: 60_000
		};
	});
}

type FollowChange = { playlistId: string; follow: boolean };

/** Follows or unfollows, updating the button right away. */
export function createFollowMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ playlistId, follow }: FollowChange) => {
			const params = { params: { path: { playlist_id: playlistId } } };
			if (follow) {
				unwrap(await api.PUT('/api/v0/playlist-follows/{playlist_id}', params));
				return;
			}
			const { response, error } = await api.DELETE(
				'/api/v0/playlist-follows/{playlist_id}',
				params
			);
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async ({ playlistId, follow }: FollowChange) => {
			const key = followKeys.state(playlistId);
			await client.cancelQueries({ queryKey: key });
			const previous = client.getQueryData<boolean>(key);
			client.setQueryData(key, follow);
			return { previous };
		},
		onError: (_error: Error, { playlistId }: FollowChange, context?: { previous?: boolean }) => {
			client.setQueryData(followKeys.state(playlistId), context?.previous);
		},
		onSettled: () => {
			void client.invalidateQueries({ queryKey: followKeys.all });
		}
	}));
}

/** Resets the playlist's new-video count, once its page was opened. */
export function useMarkSeen() {
	const client = useQueryClient();
	return async (playlistId: string) => {
		const { response } = await api
			.PUT('/api/v0/playlist-follows/{playlist_id}/seen', {
				params: { path: { playlist_id: playlistId } }
			})
			.catch(() => ({ response: undefined }));
		if (!response?.ok) return;
		client.setQueryData<FollowedPlaylist[]>(followKeys.list, (list) =>
			list?.map((p) => (p.id === playlistId ? { ...p, newVideoCount: 0 } : p))
		);
	};
}

/** Copies a YouTube playlist into a new local playlist of the signed-in user. */
export function createImportPlaylistMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (playlistId: string): Promise<Playlist> =>
			toPlaylist(
				unwrap(
					await api.POST('/api/v0/playlists/import', {
						params: { header: { 'Accept-Language': getLocale() } },
						body: { playlist_id: playlistId }
					})
				)
			),
		onSettled: () => {
			void client.invalidateQueries({ queryKey: playlistKeys.lists });
		}
	}));
}
