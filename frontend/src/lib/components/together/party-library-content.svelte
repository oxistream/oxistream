<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import HistoryIcon from '@lucide/svelte/icons/history';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import { createContinueWatchingQuery } from '#lib/api/history.js';
	import { createLikedVideosQuery } from '#lib/api/likes.js';
	import { createPlaylistQuery, createPlaylistsQuery } from '#lib/api/playlists.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { MAX_BATCH } from '#lib/together/connection.svelte.js';
	import type { VideoSummary } from '#lib/video.js';
	import PartyPick from './party-pick.svelte';

	type Source =
		{ kind: 'continue' } | { kind: 'liked' } | { kind: 'playlist'; id: string; title: string };

	let {
		queued,
		onadd,
		onaddmany
	}: {
		/** Video IDs already playing or queued. */
		queued: Set<string>;
		onadd: (videoId: string) => void;
		onaddmany: (videoIds: string[]) => void;
	} = $props();

	const locale = getLocale();
	let source = $state<Source | null>(null);
	let sent = $state(new Set<string>());
	let queueing = $state(false);

	$effect(() => {
		void queued;
		sent = new Set();
	});

	const playlists = createPlaylistsQuery();
	const continueWatching = createContinueWatchingQuery();
	const liked = createLikedVideosQuery(() => source?.kind === 'liked');
	const playlist = createPlaylistQuery(() => (source?.kind === 'playlist' ? source.id : null));

	const list = $derived.by((): { videos: VideoSummary[]; pending: boolean; error: boolean } => {
		switch (source?.kind) {
			case 'continue':
				return {
					videos: continueWatching.data ?? [],
					pending: continueWatching.isPending,
					error: continueWatching.isError
				};
			case 'liked':
				return {
					videos: liked.data?.pages.flatMap((page) => page.items) ?? [],
					pending: liked.isPending,
					error: liked.isError
				};
			case 'playlist':
				return {
					videos: playlist.data?.items ?? [],
					pending: playlist.isPending,
					error: playlist.isError
				};
			default:
				return { videos: [], pending: false, error: false };
		}
	});
	const title = $derived(
		source?.kind === 'continue'
			? m.home_continue_watching()
			: source?.kind === 'liked'
				? m.liked_title()
				: (source?.title ?? '')
	);

	function pick(video: VideoSummary) {
		sent = new Set([...sent, video.id]);
		onadd(video.id);
	}

	async function addAll() {
		if (queueing) return;
		queueing = true;
		try {
			// Liked videos come in pages; load as many as one batch can take.
			while (source?.kind === 'liked' && liked.hasNextPage && list.videos.length < MAX_BATCH) {
				await liked.fetchNextPage();
			}
			const ids = list.videos.map((video) => video.id);
			sent = new Set([...sent, ...ids.slice(0, MAX_BATCH)]);
			onaddmany(ids);
		} finally {
			queueing = false;
		}
	}
</script>

{#snippet sourceButton(
	label: string,
	count: number | undefined,
	onclick: () => void,
	Icon: typeof HeartIcon
)}
	<button
		type="button"
		class="flex h-8 w-full items-center gap-2 rounded-md px-2 text-start text-sm outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"
		{onclick}
	>
		<Icon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
		<span class="min-w-0 flex-1 truncate">{label}</span>
		{#if count !== undefined}
			<span class="text-xs text-muted-foreground tabular-nums">{formatNumber(count, locale)}</span>
		{/if}
		<ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
	</button>
{/snippet}

<div class="flex max-h-[min(28rem,70svh)] flex-col" data-testid="party-library">
	{#if !source}
		<div class="flex flex-col overflow-y-auto">
			{@render sourceButton(
				m.home_continue_watching(),
				undefined,
				() => (source = { kind: 'continue' }),
				HistoryIcon
			)}
			{@render sourceButton(
				m.liked_title(),
				undefined,
				() => (source = { kind: 'liked' }),
				HeartIcon
			)}
			{#if playlists.isPending}
				<Skeleton class="mx-2 my-1.5 h-5" />
			{:else if playlists.data?.length}
				<p class="flex h-7 items-center px-2 text-xs font-medium text-muted-foreground">
					{m.nav_playlists()}
				</p>
				{#each playlists.data as item (item.id)}
					{@render sourceButton(
						item.title,
						item.itemCount,
						() => (source = { kind: 'playlist', id: item.id, title: item.title }),
						ListVideoIcon
					)}
				{/each}
			{/if}
		</div>
	{:else}
		<div class="flex shrink-0 items-center gap-1 pb-1">
			<Button
				variant="ghost"
				size="icon-sm"
				aria-label={m.party_library_back()}
				title={m.party_library_back()}
				onclick={() => (source = null)}
			>
				<ChevronLeftIcon />
			</Button>
			<p class="min-w-0 flex-1 truncate text-sm font-medium">{title}</p>
			<Button
				variant="outline"
				size="xs"
				class="shrink-0 whitespace-nowrap"
				disabled={list.videos.length === 0 || queueing}
				onclick={addAll}
				data-testid="party-library-all"
			>
				<ListPlusIcon />
				{m.party_library_all()}
			</Button>
		</div>
		<div class="flex min-h-0 flex-col overflow-y-auto">
			{#if list.pending}
				{#each { length: 3 }, i (i)}
					<div class="flex items-center gap-2 p-1">
						<Skeleton class="aspect-video w-20 rounded-md" />
						<Skeleton class="h-3 flex-1" />
					</div>
				{/each}
			{:else if list.error}
				<p class="px-2 py-1.5 text-xs text-muted-foreground">{m.party_library_error()}</p>
			{:else if list.videos.length === 0}
				<p class="px-2 py-1.5 text-xs text-muted-foreground">{m.party_library_empty()}</p>
			{:else}
				<ul class="flex flex-col">
					{#each list.videos as video, i (`${i}:${video.id}`)}
						<li>
							<PartyPick
								{video}
								queued={queued.has(video.id) || sent.has(video.id)}
								onpick={pick}
							/>
						</li>
					{/each}
				</ul>
				{#if source.kind === 'liked' && liked.hasNextPage}
					<Button
						variant="ghost"
						size="xs"
						class="mt-1 self-center"
						disabled={liked.isFetchingNextPage}
						onclick={() => liked.fetchNextPage()}
					>
						{m.party_library_more()}
					</Button>
				{/if}
			{/if}
		</div>
	{/if}
</div>
