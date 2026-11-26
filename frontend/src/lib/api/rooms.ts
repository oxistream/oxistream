import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { session } from '#lib/session.svelte.js';
import { api } from './client';
import { isNotFound, pickImage, unwrap } from './mapping';
import type { components } from './schema';

type Schemas = components['schemas'];
export type RoomInfo = Schemas['RoomInfo'];

export type RoomListItem = {
	id: string;
	participantCount: number;
	queueCount: number;
	currentTitle?: string;
	thumbnailUrl?: string;
	createdAt: string;
	activeAt: string;
};

export function toRoomListItem(r: Schemas['RoomListItem']): RoomListItem {
	return {
		id: r.id,
		participantCount: r.participant_count,
		queueCount: r.queue_count,
		currentTitle: r.current_title ?? undefined,
		thumbnailUrl: pickImage(r.thumbnails, 320),
		createdAt: r.created_at,
		activeAt: r.active_at
	};
}

export const roomKeys = {
	list: ['rooms', 'list'] as const,
	detail: (id: string) => ['rooms', 'detail', id] as const
};

/** The signed-in user's open rooms. */
export function createRoomsQuery() {
	return createQuery(() => ({
		queryKey: roomKeys.list,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/rooms', { signal })).map(toRoomListItem),
		enabled: session.user !== null,
		staleTime: 10_000
	}));
}

export function createRoomQuery(id: () => string) {
	return createQuery(() => {
		const roomId = id();
		return {
			queryKey: roomKeys.detail(roomId),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(await api.GET('/api/v0/rooms/{id}', { params: { path: { id: roomId } }, signal })),
			// Ownership depends on the session.
			enabled: session.ready,
			retry: (count: number, error: Error) => !isNotFound(error) && count < 2,
			staleTime: 60_000
		};
	});
}

export function createCreateRoomMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (videoId?: string) =>
			unwrap(await api.POST('/api/v0/rooms', { body: { video_id: videoId } })),
		onSettled: () => void client.invalidateQueries({ queryKey: roomKeys.list })
	}));
}

export function createCloseRoomMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { response, error } = await api.DELETE('/api/v0/rooms/{id}', {
				params: { path: { id } }
			});
			if (!response.ok) unwrap({ error, response });
			return id;
		},
		onSettled: () => void client.invalidateQueries({ queryKey: roomKeys.list })
	}));
}
