<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import { goto } from '$app/navigation';
	import { createLikedVideosQuery, mergeLikedVideos } from '#lib/api/likes.js';
	import { createFollowedPlaylistsQuery } from '#lib/api/playlist-follows.js';
	import {
		createCreatePlaylistMutation,
		createPlaylistsQuery,
		type PlaylistFields
	} from '#lib/api/playlists.js';
	import LikedPlaylistCard from '#lib/components/like/liked-playlist-card.svelte';
	import FollowedPlaylistCard from '#lib/components/playlist/followed-playlist-card.svelte';
	import { playlistHref } from '#lib/components/playlist/links.js';
	import PlaylistCard from '#lib/components/playlist/playlist-card.svelte';
	import PlaylistFormDialog from '#lib/components/playlist/playlist-form-dialog.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const playlists = createPlaylistsQuery();
	const followed = createFollowedPlaylistsQuery();
	const likedVideos = createLikedVideosQuery();
	const create = createCreatePlaylistMutation();
	let createOpen = $state(false);

	const followedList = $derived(followed.data ?? []);
	const liked = $derived(mergeLikedVideos(likedVideos.data));

	async function submit(fields: PlaylistFields) {
		const playlist = await create.mutateAsync(fields);

		await goto(playlistHref(playlist.id));
	}
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_playlists()}>
		{#snippet actions()}
			{#if session.user && playlists.data}
				<Button size="sm" onclick={() => (createOpen = true)}>
					<PlusIcon />
					{m.playlist_new()}
				</Button>
			{/if}
		{/snippet}
	</PageHeader>

	{#if !session.ready}
		<VideoGrid loading skeletonCount={8} showChannel={false} />
	{:else if !session.user}
		<EmptyState
			icon={ListVideoIcon}
			title={m.empty_playlists_title()}
			description={m.empty_sign_in_hint()}
		>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{:else if playlists.isPending || (followed.isPending && followed.failureCount === 0)}
		<VideoGrid loading skeletonCount={8} showChannel={false} />
	{:else if playlists.isError}
		<ErrorState
			title={m.playlist_error_load()}
			description={m.error_load_body()}
			onretry={() => void playlists.refetch()}
		/>
	{:else}
		<section class="@container flex flex-col gap-3" aria-labelledby="own-playlists">
			<h2 id="own-playlists" class="text-base font-semibold tracking-tight">
				{m.playlist_section_own()}
			</h2>
			<ul class="video-list" data-layout="grid" aria-labelledby="own-playlists">
				<li class="min-w-0">
					<LikedPlaylistCard count={liked?.total} thumbnailUrl={liked?.items[0]?.thumbnailUrl} />
				</li>
				{#each playlists.data as playlist (playlist.id)}
					<li class="min-w-0"><PlaylistCard {playlist} /></li>
				{/each}
			</ul>
			{#if playlists.data.length === 0}
				<p class="text-sm text-muted-foreground">{m.playlist_own_empty()}</p>
			{/if}
		</section>
		{#if followedList.length > 0}
			<section class="@container flex flex-col gap-3" aria-labelledby="followed-playlists">
				<h2 id="followed-playlists" class="text-base font-semibold tracking-tight">
					{m.playlist_section_followed()}
				</h2>
				<ul class="video-list" data-layout="grid" aria-labelledby="followed-playlists">
					{#each followedList as playlist (playlist.id)}
						<li class="min-w-0"><FollowedPlaylistCard {playlist} /></li>
					{/each}
				</ul>
			</section>
		{/if}
	{/if}
</div>

{#if session.user}
	<PlaylistFormDialog bind:open={createOpen} onsubmit={submit} />
{/if}
