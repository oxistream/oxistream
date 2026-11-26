import {
	createInfiniteQuery,
	createMutation,
	createQuery,
	useQueryClient,
	type InfiniteData
} from '@tanstack/svelte-query';
import { getLocale } from '#lib/paraglide/runtime.js';
import { session } from '#lib/session.svelte.js';
import type { VideoSummary } from '#lib/video.js';
import { api } from './client';
import { toVideo, uniqueBy, unwrap } from './mapping';
import type { components } from './schema';

export type LikedVideo = VideoSummary & { likedAt: string };

export type LikedPage = { items: LikedVideo[]; total: number; continuation?: string };

export type LikedVideos = { items: LikedVideo[]; total: number };

const PAGE_SIZE = 50;

export const likeKeys = {
	all: ['likes'] as const,
	list: ['likes', 'list'] as const,
	state: (id: string) => ['likes', 'state', id] as const
};

export function toLikedPage(page: components['schemas']['LikedVideosPage']): LikedPage {
	return {
		items: page.items.map((entry) => ({ ...toVideo(entry.video), likedAt: entry.liked_at })),
		total: page.total,
		continuation: page.continuation ?? undefined
	};
}

/** All loaded pages as one list, newest first; the total comes from the latest page. */
export function mergeLikedVideos(
	data: InfiniteData<LikedPage> | undefined
): LikedVideos | undefined {
	const last = data?.pages.at(-1);
	if (!data || !last) return undefined;
	const items = uniqueBy(
		data.pages.flatMap((page) => page.items),
		(video) => video.id
	);
	return { items, total: Math.max(last.total, items.length) };
}

/** Whether the signed-in user liked the video; idle for anonymous users. */
export function createLikeStateQuery(videoId: () => string | null | undefined) {
	return createQuery(() => {
		const id = videoId() ?? '';
		return {
			queryKey: likeKeys.state(id),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/likes/{video_id}', {
						params: { path: { video_id: id } },
						signal
					})
				).liked,
			enabled: session.user !== null && id.length > 0,
			staleTime: 60_000
		};
	});
}

/** The signed-in user's liked videos, newest first, in pages. */
export function createLikedVideosQuery(enabled: () => boolean = () => true) {
	return createInfiniteQuery(() => ({
		queryKey: likeKeys.list,
		queryFn: async ({
			pageParam,
			signal
		}: {
			pageParam: string | undefined;
			signal: AbortSignal;
		}): Promise<LikedPage> =>
			toLikedPage(
				unwrap(
					await api.GET('/api/v0/likes', {
						params: { query: { continuation: pageParam, limit: PAGE_SIZE } },
						signal
					})
				)
			),
		initialPageParam: undefined as string | undefined,
		getNextPageParam: (last: LikedPage) => last.continuation,
		enabled: session.user !== null && enabled(),
		staleTime: 30_000
	}));
}

type LikeChange = { videoId: string; like: boolean };

/** Likes or unlikes a video, updating the button and the liked list right away. */
export function createLikeMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ videoId, like }: LikeChange) => {
			const params = { params: { path: { video_id: videoId } } };
			if (like) {
				unwrap(
					await api.PUT('/api/v0/likes/{video_id}', {
						params: { ...params.params, header: { 'Accept-Language': getLocale() } }
					})
				);
				return;
			}
			const { response, error } = await api.DELETE('/api/v0/likes/{video_id}', params);
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async ({ videoId, like }: LikeChange) => {
			const stateKey = likeKeys.state(videoId);
			await client.cancelQueries({ queryKey: stateKey });
			await client.cancelQueries({ queryKey: likeKeys.list });
			const previous = {
				liked: client.getQueryData<boolean>(stateKey),
				list: client.getQueryData<InfiniteData<LikedPage>>(likeKeys.list)
			};
			client.setQueryData(stateKey, like);
			const listed = previous.list?.pages.some((page) =>
				page.items.some((item) => item.id === videoId)
			);
			if (previous.list && !like && listed) {
				client.setQueryData<InfiniteData<LikedPage>>(likeKeys.list, {
					...previous.list,
					pages: previous.list.pages.map((page) => ({
						...page,
						items: page.items.filter((item) => item.id !== videoId),
						total: Math.max(0, page.total - 1)
					}))
				});
			}
			return previous;
		},
		onError: (
			_error: Error,
			{ videoId }: LikeChange,
			previous?: { liked?: boolean; list?: InfiniteData<LikedPage> }
		) => {
			client.setQueryData(likeKeys.state(videoId), previous?.liked);
			if (previous?.list) client.setQueryData(likeKeys.list, previous.list);
		},
		onSettled: () => {
			void client.invalidateQueries({ queryKey: likeKeys.list });
		}
	}));
}
