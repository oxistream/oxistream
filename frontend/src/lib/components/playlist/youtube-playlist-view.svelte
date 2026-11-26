<script lang="ts">
	import CopyPlusIcon from '@lucide/svelte/icons/copy-plus';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PlayIcon from '@lucide/svelte/icons/play';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { ApiError, isNotFound } from '#lib/api/mapping.js';
	import {
		createFollowMutation,
		createFollowStateQuery,
		createImportPlaylistMutation,
		useMarkSeen
	} from '#lib/api/playlist-follows.js';
	import { isMixId } from '#lib/api/playlist-order.js';
	import {
		createYouTubePlaylistQuery,
		mergeYouTubePlaylist,
		type LoadedYouTubePlaylist
	} from '#lib/api/youtube-playlists.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { formatDate, formatNumber, formatRelativeDay } from '#lib/format.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { authHref } from '#lib/auth/links.js';
	import { session } from '#lib/session.svelte.js';
	import CourseBadge from './course-badge.svelte';
	import FollowToggle from './follow-toggle.svelte';
	import { playlistHref, playlistWatchHref } from './links';
	import PlaylistCover from './playlist-cover.svelte';
	import PlaylistItems from './playlist-items.svelte';
	import PlaylistLayout from './playlist-layout.svelte';

	let { listId }: { listId: string } = $props();

	const locale = getLocale();
	const query = createYouTubePlaylistQuery(() => listId);
	const followState = createFollowStateQuery(() => listId);
	const followMutation = createFollowMutation();
	const importMutation = createImportPlaylistMutation();
	const markSeen = useMarkSeen();

	const signInHref = $derived(
		session.user ? undefined : authHref('/login', page.url.pathname + page.url.search)
	);
	const following = $derived(followState.data === true);

	// Once per page, and only for a follow the server reported: a new follow has nothing new yet.
	let seenId: string | undefined;
	$effect(() => {
		if (seenId === listId || !query.data || !followState.isSuccess) return;
		if (followMutation.isPending || followMutation.isSuccess) return;
		seenId = listId;
		if (followState.data) void markSeen(listId);
	});

	async function saveCopy() {
		const copy = await importMutation.mutateAsync(listId).catch(() => undefined);
		if (copy) await goto(playlistHref(copy.id));
	}

	const data = $derived<LoadedYouTubePlaylist | undefined>(mergeYouTubePlaylist(query.data));
	const mix = $derived(
		isMixId(listId) || (query.error instanceof ApiError && query.error.reason === 'mix')
	);
	const notFound = $derived(!data && query.isError && isNotFound(query.error));
	const start = $derived(data?.items[0]);
	// YouTube counts unavailable videos it does not list.
	const hidden = $derived(
		data !== undefined && !query.hasNextPage && data.items.length < data.videoCount
	);
	const title = $derived(
		data
			? pageTitle(data.title)
			: pageTitle(
					mix
						? m.playlist_mix_title()
						: notFound
							? m.playlist_not_found_title()
							: m.search_playlist()
				)
	);
</script>

<svelte:head>
	<title>{title}</title>
</svelte:head>

{#if !data}<h1 class="sr-only">{m.search_playlist()}</h1>{/if}
{#if mix}
	<EmptyState
		icon={ListVideoIcon}
		title={m.playlist_mix_title()}
		description={m.playlist_mix_body()}
	/>
{:else if query.isPending}
	<PlaylistLayout />
{:else if notFound}
	<EmptyState
		icon={ListVideoIcon}
		title={m.playlist_not_found_title()}
		description={m.playlist_unavailable_body()}
	/>
{:else if !data}
	<ErrorState
		title={m.playlist_error_load()}
		description={m.error_load_body()}
		onretry={() => void query.refetch()}
	/>
{:else}
	<PlaylistLayout>
		{#snippet hero()}
			<PlaylistCover src={data.thumbnailUrl} count={data.videoCount} class="max-w-md" />
			<div class="flex min-w-0 flex-col gap-1.5">
				{#if data.isCourse}<CourseBadge />{/if}
				<h1 class="text-xl font-semibold tracking-tight text-pretty break-words">{data.title}</h1>
				<p class="flex flex-wrap items-center gap-x-1.5 text-sm text-muted-foreground">
					{#if data.channel}
						<a
							href={resolve('/channel/[id]', { id: data.channel.id })}
							class="rounded-sm font-medium text-foreground transition-colors duration-(--motion-in) ease-ui hover:text-primary"
						>
							{data.channel.name}
						</a>
						<span aria-hidden="true">·</span>
					{/if}
					<span>{m.playlist_source_youtube()}</span>
					<span aria-hidden="true">·</span>
					<span class="tabular">
						{m.playlist_video_count({
							count: data.videoCount,
							formatted: formatNumber(data.videoCount, locale)
						})}
					</span>
					{#if data.lastUpdate}
						<span aria-hidden="true">·</span>
						<span title={formatDate(`${data.lastUpdate}T12:00:00`, locale)}>
							{m.playlist_updated({ time: formatRelativeDay(data.lastUpdate, locale) })}
						</span>
					{:else if data.lastUpdateText}
						<span aria-hidden="true">·</span>
						<span>{data.lastUpdateText}</span>
					{/if}
				</p>
				{#if data.description}
					<p class="mt-1 line-clamp-6 text-sm text-pretty break-words whitespace-pre-line">
						{data.description}
					</p>
				{/if}
			</div>
			<div class="flex flex-wrap gap-2">
				{#if start}
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
					<Button href={playlistWatchHref(start.id, data.id)}>
						<PlayIcon class="fill-current" />
						{m.playlist_play_all()}
					</Button>
				{/if}
				<FollowToggle
					{following}
					{signInHref}
					loading={session.user !== null && followState.isPending}
					onclick={() => followMutation.mutate({ playlistId: data.id, follow: !following })}
				/>
				{#if signInHref}
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved by authHref -->
					<Button href={signInHref} variant="outline">
						<CopyPlusIcon />
						{m.playlist_copy()}
					</Button>
				{:else}
					<Button
						variant="outline"
						disabled={importMutation.isPending}
						aria-busy={importMutation.isPending || undefined}
						data-testid="copy-playlist"
						onclick={saveCopy}
					>
						<CopyPlusIcon />
						{importMutation.isPending ? m.playlist_copying() : m.playlist_copy()}
					</Button>
				{/if}
			</div>
			{#if importMutation.isError}
				<p class="text-sm font-medium text-destructive" role="alert">
					{m.playlist_copy_error()}
				</p>
			{/if}
		{/snippet}

		{#if data.items.length === 0 && !query.hasNextPage}
			<EmptyState
				icon={ListVideoIcon}
				title={m.playlist_empty_title()}
				description={hidden ? m.playlist_hidden_videos() : undefined}
				headingLevel={2}
			/>
		{:else}
			<PlaylistItems playlistId={data.id} items={data.items} />
			<InfiniteLoader
				hasMore={query.hasNextPage}
				loading={query.isFetchingNextPage}
				failed={query.isFetchNextPageError}
				onload={() => void query.fetchNextPage()}
				endLabel={hidden ? m.playlist_hidden_videos() : undefined}
			/>
		{/if}
	</PlaylistLayout>
{/if}
