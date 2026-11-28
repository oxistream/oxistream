import {
	createInfiniteQuery,
	createMutation,
	createQueries,
	createQuery,
	keepPreviousData,
	useQueryClient,
	type InfiniteData,
	type QueryClient,
	type QueryKey
} from '@tanstack/svelte-query';
import type { ProgressReport } from '#lib/components/player/progress.js';
import {
	readLocalProgress,
	saveLocalProgress,
	type LocalVideo
} from '#lib/components/watch/local-progress.js';
import { session } from '#lib/session.svelte.js';
import { api } from './client';
import {
	chunk,
	progressRatio,
	toHistoryVideo,
	toProgress,
	type HistoryVideo,
	type WatchProgress
} from './history-list';
import { pickImage, uniqueBy, unwrap } from './mapping';
import type { Retention } from './retention';
import type { components } from './schema';
import type { VideoDetails } from './video';

export type HistorySettings = components['schemas']['HistorySettings'];
type HistorySettingsUpdate = components['schemas']['HistorySettingsUpdate'];

export type HistoryPage = { items: HistoryVideo[]; continuation?: string };

const HISTORY_PAGE_SIZE = 30;
const CONTINUE_LIMIT = 12;
const BATCH_SIZE = 50;
const PROGRESS_STALE_MS = 30_000;

export const historyKeys = {
	all: ['history'] as const,
	list: ['history', 'list'] as const,
	search: (q: string) => ['history', 'list', q] as const,
	continue: ['history', 'continue'] as const,
	settings: ['history', 'settings'] as const,
	progress: ['progress'] as const,
	video: (id: string) => ['progress', 'video', id] as const,
	batch: (ids: string[]) => ['progress', 'batch', ids.join(',')] as const
};

/** Server-side progress of one video, to resume it; idle for anonymous users. */
export function createVideoProgressQuery(videoId: () => string | null) {
	return createQuery(() => {
		const id = videoId() ?? '';
		return {
			queryKey: historyKeys.video(id),
			queryFn: async ({ signal }: { signal: AbortSignal }): Promise<WatchProgress | null> => {
				const state = unwrap(
					await api.GET('/api/v0/progress/{video_id}', {
						params: { path: { video_id: id } },
						signal
					})
				);
				return state.progress ? toProgress(state.progress) : null;
			},
			enabled: session.user !== null && id.length > 0,
			staleTime: PROGRESS_STALE_MS
		};
	});
}

async function fetchBatch(ids: string[], signal: AbortSignal): Promise<Record<string, number>> {
	const list = unwrap(
		await api.GET('/api/v0/progress', { params: { query: { ids: ids.join(',') } }, signal })
	);
	return Object.fromEntries(list.map((p) => [p.video_id, progressRatio(toProgress(p))]));
}

/**
 * Watched share of the given videos: from the server for signed-in users, from this browser's
 * storage for anonymous ones.
 */
export function createProgressLookup(videoIds: () => string[]) {
	const server = createQueries(() => ({
		queries: (session.user
			? chunk(
					uniqueBy(videoIds(), (id) => id),
					BATCH_SIZE
				)
			: []
		).map((part) => ({
			queryKey: historyKeys.batch(part),
			queryFn: ({ signal }: { signal: AbortSignal }) => fetchBatch(part, signal),
			staleTime: PROGRESS_STALE_MS
		})),
		combine: (results) => ({
			data: Object.assign({}, ...results.map((r) => r.data ?? {})) as Record<string, number>
		})
	}));
	return {
		get: (id: string): number | undefined => {
			if (session.user) return server.data[id];
			if (!session.ready) return undefined;
			const local = readLocalProgress(id);
			return local ? progressRatio(local) : undefined;
		}
	};
}

async function fetchHistoryPage(
	q: string,
	continuation: string | undefined,
	signal: AbortSignal
): Promise<HistoryPage> {
	const page = unwrap(
		await api.GET('/api/v0/history', {
			params: { query: { limit: HISTORY_PAGE_SIZE, continuation, q: q || undefined } },
			signal
		})
	);
	return {
		items: page.items.map(toHistoryVideo),
		continuation: page.continuation ?? undefined
	};
}

/** Watched videos, most recent first; with a search term, only those matching it. */
export function createHistoryQuery(search: () => string = () => '') {
	return createInfiniteQuery(() => {
		const q = search().trim();
		return {
			queryKey: historyKeys.search(q),
			queryFn: ({ pageParam, signal }: { pageParam: string | undefined; signal: AbortSignal }) =>
				fetchHistoryPage(q, pageParam, signal),
			initialPageParam: undefined as string | undefined,
			getNextPageParam: (last: HistoryPage) => last.continuation,
			// Keeps the list while the next search loads.
			placeholderData: keepPreviousData,
			enabled: session.user !== null,
			staleTime: PROGRESS_STALE_MS
		};
	});
}

/** Started but unfinished videos for the home page. */
export function createContinueWatchingQuery() {
	return createQuery(() => ({
		queryKey: historyKeys.continue,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(
				await api.GET('/api/v0/history/continue', {
					params: { query: { limit: CONTINUE_LIMIT } },
					signal
				})
			).map(toHistoryVideo),
		enabled: session.user !== null,
		staleTime: PROGRESS_STALE_MS
	}));
}

export function createHistorySettingsQuery() {
	return createQuery(() => ({
		queryKey: historyKeys.settings,
		queryFn: async ({ signal }: { signal: AbortSignal }): Promise<HistorySettings> =>
			unwrap(await api.GET('/api/v0/history/settings', { signal })),
		enabled: session.user !== null,
		staleTime: 5 * 60_000
	}));
}

function sendReport(videoId: string, report: ProgressReport) {
	const body = {
		video_id: videoId,
		position_seconds: report.position,
		duration_seconds: report.duration,
		finished: report.finished
	};
	// The page may be going away; a beacon still gets through. Plain text needs no preflight.
	if (report.reason === 'unload' && typeof navigator.sendBeacon === 'function') {
		const blob = new Blob([JSON.stringify(body)], { type: 'text/plain' });
		if (navigator.sendBeacon('/api/v0/progress', blob)) return;
	}
	void api.POST('/api/v0/progress', { body }).catch(() => undefined);
}

/** The loaded video's card details, kept with anonymous progress for the home page. */
function cachedLocalVideo(
	client: ReturnType<typeof useQueryClient>,
	videoId: string
): LocalVideo | undefined {
	const details = client
		.getQueriesData<VideoDetails>({ queryKey: ['video', videoId] })
		.find(([, data]) => data)?.[1];
	if (!details) return undefined;
	return {
		title: details.title,
		channel: { id: details.channel.id, name: details.channel.name },
		durationSeconds: details.is_live ? undefined : (details.duration_seconds ?? undefined),
		isLive: details.is_live,
		thumbnailUrl: pickImage(details.thumbnails, 480)
	};
}

/**
 * Stores player progress: on the server for signed-in users, in this browser otherwise. Lists
 * showing progress are marked stale, so they refetch when shown next.
 */
export function createProgressReporter() {
	const client = useQueryClient();
	const paused = createHistorySettingsQuery();
	return (videoId: string) => (report: ProgressReport) => {
		if (!session.user) {
			saveLocalProgress(videoId, report, Date.now(), cachedLocalVideo(client, videoId));
			return;
		}
		if (paused.data?.paused) return;
		sendReport(videoId, report);
		const saved: WatchProgress = {
			position: report.position,
			duration: report.duration,
			finished: report.finished,
			updatedAt: new Date().toISOString()
		};
		client.setQueryData(historyKeys.video(videoId), saved);
		void client.invalidateQueries({
			predicate: (query) =>
				(query.queryKey[0] === historyKeys.progress[0] && query.queryKey[1] === 'batch') ||
				query.queryKey[0] === historyKeys.all[0],
			refetchType: 'none'
		});
	};
}

type HistoryData = InfiniteData<HistoryPage, string | undefined>;

function withoutVideo(data: HistoryData | undefined, videoId: string) {
	if (!data) return data;
	return {
		...data,
		pages: data.pages.map((page) => ({
			...page,
			items: page.items.filter((video) => video.id !== videoId)
		}))
	};
}

function invalidateProgress(client: QueryClient) {
	void client.invalidateQueries({ queryKey: historyKeys.all });
	void client.invalidateQueries({ queryKey: historyKeys.progress });
}

/** Removes one video from the history, right away in the list. */
export function createRemoveFromHistoryMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (videoId: string) => {
			const { response, error } = await api.DELETE('/api/v0/history/{video_id}', {
				params: { path: { video_id: videoId } }
			});
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async (videoId: string) => {
			await client.cancelQueries({ queryKey: historyKeys.list });
			const previous = client.getQueriesData<HistoryData>({ queryKey: historyKeys.list });
			client.setQueriesData<HistoryData>({ queryKey: historyKeys.list }, (data) =>
				withoutVideo(data, videoId)
			);
			return { previous };
		},
		onError: (
			_error: Error,
			_videoId: string,
			context?: { previous?: [QueryKey, HistoryData | undefined][] }
		) => {
			for (const [key, data] of context?.previous ?? []) client.setQueryData(key, data);
		},
		onSettled: () => invalidateProgress(client)
	}));
}

/** Hides one video from "Continue watching" until it is watched again; the history keeps it. */
export function createDismissContinueMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (videoId: string) => {
			const { response, error } = await api.DELETE('/api/v0/history/continue/{video_id}', {
				params: { path: { video_id: videoId } }
			});
			if (!response.ok) unwrap({ error, response });
		},
		onMutate: async (videoId: string) => {
			await client.cancelQueries({ queryKey: historyKeys.continue });
			const previous = client.getQueryData<HistoryVideo[]>(historyKeys.continue);
			client.setQueryData(
				historyKeys.continue,
				previous?.filter((video) => video.id !== videoId)
			);
			return { previous };
		},
		onError: (_error: Error, _videoId: string, context?: { previous?: HistoryVideo[] }) => {
			if (context?.previous) client.setQueryData(historyKeys.continue, context.previous);
		},
		onSettled: () => void client.invalidateQueries({ queryKey: historyKeys.continue })
	}));
}

export function createClearHistoryMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async () => {
			const { response, error } = await api.DELETE('/api/v0/history');
			if (!response.ok) unwrap({ error, response });
		},
		onSuccess: () => {
			client.setQueriesData<HistoryData>({ queryKey: historyKeys.list }, () => ({
				pages: [{ items: [] }],
				pageParams: [undefined]
			}));
			client.setQueryData(historyKeys.continue, []);
		},
		onSettled: () => invalidateProgress(client)
	}));
}

function createHistorySettingsMutation<T>(
	body: (value: T) => HistorySettingsUpdate,
	onSaved?: (client: QueryClient) => void
) {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (value: T) =>
			unwrap(await api.PUT('/api/v0/history/settings', { body: body(value) })),
		onMutate: async (value: T) => {
			await client.cancelQueries({ queryKey: historyKeys.settings });
			const previous = client.getQueryData<HistorySettings>(historyKeys.settings);
			if (previous) client.setQueryData(historyKeys.settings, { ...previous, ...body(value) });
			return { previous };
		},
		onError: (_error: Error, _value: T, context?: { previous?: HistorySettings }) => {
			client.setQueryData(historyKeys.settings, context?.previous);
		},
		onSuccess: (settings: HistorySettings) => {
			client.setQueryData(historyKeys.settings, settings);
			onSaved?.(client);
		}
	}));
}

export function createHistoryPausedMutation() {
	return createHistorySettingsMutation((paused: boolean) => ({ paused }));
}

/** A shorter retention hides older entries right away, so lists refetch. */
export function createHistoryRetentionMutation() {
	return createHistorySettingsMutation(
		(retention: Retention) => ({ retention }),
		invalidateProgress
	);
}
