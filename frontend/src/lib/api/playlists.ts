import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { session } from '#lib/session.svelte.js';
import type { VideoSummary } from '#lib/video.js';
import { api } from './client';
import { isNotFound, pickImage, toVideo, unwrap } from './mapping';
import { isLocalPlaylistId, moveItem } from './playlist-order';
import type { components } from './schema';

type Schemas = components['schemas'];
export type PlaylistVisibility = Schemas['PlaylistVisibility'];

export type Playlist = {
	id: string;
	title: string;
	description: string;
	visibility: PlaylistVisibility;
	itemCount: number;
	thumbnailUrl?: string;
	/** Where the owner left off; only known to the owner. */
	lastVideoId?: string;
	/** Set when the list was loaded for a video. */
	containsVideo?: boolean;
	createdAt: string;
	updatedAt: string;
};

export type PlaylistVideo = VideoSummary & { addedAt: string };

export type PlaylistDetails = Playlist & {
	owner: string;
	isOwner: boolean;
	items: PlaylistVideo[];
};

export type PlaylistFields = {
	title: string;
	description: string;
	visibility: PlaylistVisibility;
};

export function toPlaylist(p: Schemas['Playlist']): Playlist {
	return {
		id: p.id,
		title: p.title,
		description: p.description,
		visibility: p.visibility,
		itemCount: p.item_count,
		thumbnailUrl: pickImage(p.thumbnails, 320),
		lastVideoId: p.last_video_id ?? undefined,
		containsVideo: p.contains_video ?? undefined,
		createdAt: p.created_at,
		updatedAt: p.updated_at
	};
}

export function toPlaylistDetails(p: Schemas['PlaylistDetails']): PlaylistDetails {
	return {
		...toPlaylist(p),
		owner: p.owner,
		isOwner: p.is_owner,
		items: p.items.map((entry) => ({ ...toVideo(entry.video), addedAt: entry.added_at }))
	};
}

export const playlistKeys = {
	all: ['playlists'] as const,
	lists: ['playlists', 'list'] as const,
	list: (videoId?: string) => ['playlists', 'list', videoId ?? null] as const,
	detail: (id: string) => ['playlists', 'detail', id] as const
};

/** The signed-in user's playlists; with a video, each tells whether it contains it. */
export function createPlaylistsQuery(videoId: () => string | undefined = () => undefined) {
	return createQuery(() => {
		const video = videoId();
		return {
			queryKey: playlistKeys.list(video),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/playlists', {
						params: { query: { video_id: video } },
						signal
					})
				).map(toPlaylist),
			enabled: session.user !== null,
			staleTime: 30_000
		};
	});
}

export function createPlaylistQuery(id: () => string | null | undefined) {
	return createQuery(() => {
		const listId = id() ?? '';
		return {
			queryKey: playlistKeys.detail(listId),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				toPlaylistDetails(
					unwrap(
						await api.GET('/api/v0/playlists/{id}', {
							params: { path: { id: listId } },
							signal
						})
					)
				),
			// Waits for the session, since ownership changes what the response contains.
			enabled: session.ready && isLocalPlaylistId(listId),
			retry: (count: number, error: Error) => !isNotFound(error) && count < 2,
			staleTime: 30_000
		};
	});
}

function invalidateLists(client: ReturnType<typeof useQueryClient>) {
	void client.invalidateQueries({ queryKey: playlistKeys.lists });
}

export function createCreatePlaylistMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (fields: PlaylistFields) =>
			toPlaylist(unwrap(await api.POST('/api/v0/playlists', { body: fields }))),
		onSettled: () => invalidateLists(client)
	}));
}

export function createUpdatePlaylistMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id, fields }: { id: string; fields: Partial<PlaylistFields> }) =>
			toPlaylist(
				unwrap(
					await api.PATCH('/api/v0/playlists/{id}', {
						params: { path: { id } },
						body: fields
					})
				)
			),
		onSuccess: (playlist: Playlist) => {
			client.setQueryData<PlaylistDetails>(playlistKeys.detail(playlist.id), (old) =>
				old ? { ...old, ...playlist, lastVideoId: old.lastVideoId } : old
			);
		},
		onSettled: () => invalidateLists(client)
	}));
}

export function createDeletePlaylistMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { response, error } = await api.DELETE('/api/v0/playlists/{id}', {
				params: { path: { id } }
			});
			if (!response.ok) unwrap({ error, response });
			return id;
		},
		// The detail is dropped by the caller once it navigated away, so the page does not refetch it.
		onSettled: () => invalidateLists(client)
	}));
}

type ItemChange = { playlistId: string; videoId: string; add: boolean };

/** Adds or removes a video, updating checkboxes and the playlist page right away. */
export function createPlaylistItemMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ playlistId, videoId, add }: ItemChange) => {
			const params = { params: { path: { id: playlistId, video_id: videoId } } };
			if (add) {
				unwrap(await api.PUT('/api/v0/playlists/{id}/items/{video_id}', params));
				return;
			}
			const { response, error } = await api.DELETE(
				'/api/v0/playlists/{id}/items/{video_id}',
				params
			);
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async ({ playlistId, videoId, add }: ItemChange) => {
			const listKey = playlistKeys.list(videoId);
			const detailKey = playlistKeys.detail(playlistId);
			await client.cancelQueries({ queryKey: listKey });
			await client.cancelQueries({ queryKey: detailKey });
			const previous = {
				list: client.getQueryData<Playlist[]>(listKey),
				detail: client.getQueryData<PlaylistDetails>(detailKey)
			};
			if (previous.list) {
				client.setQueryData<Playlist[]>(
					listKey,
					previous.list.map((p) => (p.id === playlistId ? { ...p, containsVideo: add } : p))
				);
			}
			if (previous.detail && !add) {
				const items = previous.detail.items.filter((item) => item.id !== videoId);
				client.setQueryData<PlaylistDetails>(detailKey, {
					...previous.detail,
					items,
					itemCount: items.length
				});
			}
			return previous;
		},
		onError: (
			_error: Error,
			{ playlistId, videoId }: ItemChange,
			previous?: { list?: Playlist[]; detail?: PlaylistDetails }
		) => {
			if (previous?.list) client.setQueryData(playlistKeys.list(videoId), previous.list);
			if (previous?.detail) client.setQueryData(playlistKeys.detail(playlistId), previous.detail);
		},
		onSettled: () => {
			void client.invalidateQueries({ queryKey: playlistKeys.all });
		}
	}));
}

type Move = { playlistId: string; videoId: string; from: number; to: number };

/** Moves a video within a playlist; the list reorders before the server confirms. */
export function createMoveItemMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ playlistId, videoId, to }: Move) => {
			const { response, error } = await api.PUT(
				'/api/v0/playlists/{id}/items/{video_id}/position',
				{ params: { path: { id: playlistId, video_id: videoId } }, body: { index: to } }
			);
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async ({ playlistId, from, to }: Move) => {
			const key = playlistKeys.detail(playlistId);
			await client.cancelQueries({ queryKey: key });
			const previous = client.getQueryData<PlaylistDetails>(key);
			if (previous) {
				client.setQueryData<PlaylistDetails>(key, {
					...previous,
					items: moveItem(previous.items, from, to)
				});
			}
			return { previous };
		},
		onError: (_error: Error, { playlistId }: Move, context?: { previous?: PlaylistDetails }) => {
			if (context?.previous) client.setQueryData(playlistKeys.detail(playlistId), context.previous);
		},
		onSettled: (_data: unknown, _error: Error | null, { playlistId }: Move) => {
			void client.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) });
			invalidateLists(client);
		}
	}));
}

/** Remembers the video the owner is playing, for "Continue" on the playlist page. */
export function useLastPlayed() {
	const client = useQueryClient();
	return async (playlistId: string, videoId: string) => {
		client.setQueryData<PlaylistDetails>(playlistKeys.detail(playlistId), (old) =>
			old ? { ...old, lastVideoId: videoId } : old
		);
		await api
			.PUT('/api/v0/playlists/{id}/last-played', {
				params: { path: { id: playlistId } },
				body: { video_id: videoId }
			})
			.catch(() => undefined);
	};
}
