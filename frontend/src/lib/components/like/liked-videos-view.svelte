<script lang="ts">
	import PlayIcon from '@lucide/svelte/icons/play';
	import ThumbsUpIcon from '@lucide/svelte/icons/thumbs-up';
	import { createLikeMutation, createLikedVideosQuery, mergeLikedVideos } from '#lib/api/likes.js';
	import { LIKED_LIST_ID } from '#lib/api/playlist-order.js';
	import { playlistWatchHref } from '#lib/components/playlist/links.js';
	import PlaylistCover from '#lib/components/playlist/playlist-cover.svelte';
	import PlaylistItems from '#lib/components/playlist/playlist-items.svelte';
	import PlaylistLayout from '#lib/components/playlist/playlist-layout.svelte';
	import VisibilityLabel from '#lib/components/playlist/visibility-label.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { formatNumber } from '#lib/format.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	const locale = getLocale();
	const query = createLikedVideosQuery();
	const unlike = createLikeMutation();

	const data = $derived(mergeLikedVideos(query.data));
	const start = $derived(data?.items[0]);
</script>

<svelte:head>
	<title>{pageTitle(m.liked_title())}</title>
</svelte:head>

{#if !data}<h1 class="sr-only">{m.liked_title()}</h1>{/if}
{#if !session.ready || (session.user && query.isPending)}
	<PlaylistLayout />
{:else if !session.user}
	<EmptyState icon={ThumbsUpIcon} title={m.liked_title()} description={m.liked_sign_in()}>
		{#snippet actions()}<SignInActions />{/snippet}
	</EmptyState>
{:else if !data}
	<ErrorState
		title={m.liked_error_load()}
		description={m.error_load_body()}
		onretry={() => void query.refetch()}
	/>
{:else}
	<PlaylistLayout>
		{#snippet hero()}
			<PlaylistCover
				src={start?.thumbnailUrl}
				count={data.total}
				icon={ThumbsUpIcon}
				class="max-w-md"
			/>
			<div class="flex min-w-0 flex-col gap-1.5">
				<h1 class="text-xl font-semibold tracking-tight text-pretty break-words sm:text-2xl">
					{m.liked_title()}
				</h1>
				<p class="flex flex-wrap items-center gap-x-1.5 text-sm text-muted-foreground">
					<span class="font-medium text-foreground">{session.user?.username}</span>
					<span aria-hidden="true">·</span>
					<VisibilityLabel visibility="private" />
					<span aria-hidden="true">·</span>
					<span class="tabular">
						{m.playlist_video_count({
							count: data.total,
							formatted: formatNumber(data.total, locale)
						})}
					</span>
				</p>
			</div>
			{#if start}
				<div class="flex flex-wrap gap-2">
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
					<Button href={playlistWatchHref(start.id, LIKED_LIST_ID)}>
						<PlayIcon class="fill-current" />
						{m.playlist_play_all()}
					</Button>
				</div>
			{/if}
		{/snippet}

		{#if data.items.length === 0 && !query.hasNextPage}
			<EmptyState icon={ThumbsUpIcon} title={m.liked_empty_title()} headingLevel={2} />
		{:else}
			<PlaylistItems
				playlistId={LIKED_LIST_ID}
				items={data.items}
				editable
				reorderable={false}
				onremove={(video) => unlike.mutate({ videoId: video.id, like: false })}
			/>
			<InfiniteLoader
				hasMore={query.hasNextPage}
				loading={query.isFetchingNextPage}
				failed={query.isFetchNextPageError}
				onload={() => void query.fetchNextPage()}
			/>
		{/if}
	</PlaylistLayout>
{/if}
