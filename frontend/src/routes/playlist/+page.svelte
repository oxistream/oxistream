<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlayIcon from '@lucide/svelte/icons/play';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import Share2Icon from '@lucide/svelte/icons/share-2';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { isNotFound } from '#lib/api/mapping.js';
	import { isLikedListId, isLocalPlaylistId, resumeVideo } from '#lib/api/playlist-order.js';
	import {
		createDeletePlaylistMutation,
		createMoveItemMutation,
		createPlaylistItemMutation,
		createPlaylistQuery,
		createUpdatePlaylistMutation,
		playlistKeys,
		type PlaylistFields
	} from '#lib/api/playlists.js';
	import { playlistHref, playlistWatchHref } from '#lib/components/playlist/links.js';
	import LikedVideosView from '#lib/components/like/liked-videos-view.svelte';
	import PlaylistCover from '#lib/components/playlist/playlist-cover.svelte';
	import IconAction from '#lib/components/playlist/icon-action.svelte';
	import PlaylistFormDialog from '#lib/components/playlist/playlist-form-dialog.svelte';
	import PlaylistItems from '#lib/components/playlist/playlist-items.svelte';
	import PlaylistLayout from '#lib/components/playlist/playlist-layout.svelte';
	import VisibilityLabel from '#lib/components/playlist/visibility-label.svelte';
	import YouTubePlaylistView from '#lib/components/playlist/youtube-playlist-view.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { formatDurationLong, formatNumber } from '#lib/format.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	const locale = getLocale();
	const client = useQueryClient();
	const listId = $derived(page.url.searchParams.get('list'));
	const local = $derived(isLocalPlaylistId(listId));
	const liked = $derived(isLikedListId(listId));
	const playlist = createPlaylistQuery(() => listId);
	const update = createUpdatePlaylistMutation();
	const remove = createDeletePlaylistMutation();
	const itemMutation = createPlaylistItemMutation();
	const moveMutation = createMoveItemMutation();

	let editOpen = $state(false);
	let deleteOpen = $state(false);
	let copied = $state(false);

	const data = $derived(playlist.data);
	const notFound = $derived(playlist.isError && isNotFound(playlist.error));
	const start = $derived(data?.items[0]);
	const resume = $derived(
		data?.isOwner && data.lastVideoId ? resumeVideo(data.items, data.lastVideoId) : undefined
	);
	const continuing = $derived(resume !== undefined && resume.id !== start?.id);
	const totalSeconds = $derived(
		data?.items.reduce((sum, video) => sum + (video.durationSeconds ?? 0), 0) ?? 0
	);
	const title = $derived(
		data
			? pageTitle(data.title)
			: pageTitle(notFound ? m.playlist_not_found_title() : m.search_playlist())
	);

	function save(fields: PlaylistFields) {
		if (!data) return Promise.resolve();
		return update.mutateAsync({ id: data.id, fields });
	}

	async function confirmDelete() {
		if (!data) return;
		const id = data.id;
		await remove.mutateAsync(id);
		deleteOpen = false;
		await goto(resolve('/feed/playlists'));
		client.removeQueries({ queryKey: playlistKeys.detail(id) });
	}

	async function share() {
		if (!data) return;
		const url = new URL(playlistHref(data.id), window.location.origin).href;
		try {
			await navigator.clipboard.writeText(url);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			window.prompt(m.playlist_share_prompt(), url);
		}
	}
</script>

<svelte:head>
	{#if !listId || local}<title>{title}</title>{/if}
</svelte:head>

<div class="page-column py-6">
	{#if !data && (local || !listId)}<h1 class="sr-only">{m.search_playlist()}</h1>{/if}
	{#if !listId}
		<EmptyState icon={ListVideoIcon} title={m.playlist_none_title()} />
	{:else if liked}
		<LikedVideosView />
	{:else if !local}
		<YouTubePlaylistView {listId} />
	{:else if playlist.isPending}
		<PlaylistLayout />
	{:else if notFound}
		<EmptyState
			icon={ListVideoIcon}
			title={m.playlist_not_found_title()}
			description={session.user ? m.playlist_not_found_body() : m.playlist_not_found_sign_in()}
		>
			{#snippet actions()}
				{#if !session.user}<SignInActions />{/if}
			{/snippet}
		</EmptyState>
	{:else if !data}
		<ErrorState
			title={m.playlist_error_load()}
			description={m.error_load_body()}
			onretry={() => void playlist.refetch()}
		/>
	{:else}
		<PlaylistLayout>
			{#snippet hero()}
				<PlaylistCover src={data.thumbnailUrl} count={data.items.length} class="max-w-md" />
				<div class="flex min-w-0 flex-col gap-1.5">
					<h1 class="text-xl font-semibold tracking-tight text-pretty break-words sm:text-2xl">
						{data.title}
					</h1>
					<p class="flex flex-wrap items-center gap-x-1.5 text-sm text-muted-foreground">
						<span class="font-medium text-foreground">{data.owner}</span>
						<span aria-hidden="true">·</span>
						<VisibilityLabel visibility={data.visibility} />
						<span aria-hidden="true">·</span>
						<span class="tabular">
							{m.playlist_video_count({
								count: data.items.length,
								formatted: formatNumber(data.items.length, locale)
							})}
						</span>
						{#if totalSeconds > 0}
							<span aria-hidden="true">·</span>
							<span class="tabular">{formatDurationLong(totalSeconds, locale)}</span>
						{/if}
					</p>
					{#if data.description}
						<p class="mt-1 text-sm text-pretty break-words whitespace-pre-line">
							{data.description}
						</p>
					{/if}
				</div>
				<div class="flex items-center gap-2">
					{#if continuing && resume}
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
						<Button href={playlistWatchHref(resume.id, data.id)} data-testid="playlist-continue">
							<PlayIcon class="fill-current" />
							{m.playlist_continue()}
						</Button>
						{#if start}
							<IconAction
								href={playlistWatchHref(start.id, data.id)}
								label={m.playlist_play_from_start()}
							>
								<RotateCcwIcon />
							</IconAction>
						{/if}
					{:else if start}
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
						<Button href={playlistWatchHref(start.id, data.id)}>
							<PlayIcon class="fill-current" />
							{m.playlist_play_all()}
						</Button>
					{/if}
					{#if data.visibility === 'unlisted'}
						<IconAction
							label={m.playlist_share()}
							tooltip={copied ? m.playlist_share_copied() : m.playlist_share()}
							onclick={share}
						>
							{#if copied}<CheckIcon />{:else}<Share2Icon />{/if}
						</IconAction>
					{/if}
					{#if data.isOwner}
						<IconAction label={m.playlist_edit()} onclick={() => (editOpen = true)}>
							<PencilIcon />
						</IconAction>
						<IconAction label={m.playlist_delete()} onclick={() => (deleteOpen = true)}>
							<Trash2Icon />
						</IconAction>
					{/if}
				</div>
				<p class="sr-only" aria-live="polite">{copied ? m.playlist_share_copied() : ''}</p>
			{/snippet}

			{#if data.items.length === 0}
				<EmptyState icon={ListVideoIcon} title={m.playlist_empty_title()} headingLevel={2} />
			{:else}
				<PlaylistItems
					playlistId={data.id}
					items={data.items}
					editable={data.isOwner}
					onmove={(video, from, to) =>
						moveMutation.mutate({ playlistId: data.id, videoId: video.id, from, to })}
					onremove={(video) =>
						itemMutation.mutate({ playlistId: data.id, videoId: video.id, add: false })}
				/>
			{/if}
		</PlaylistLayout>

		{#if data.isOwner}
			<PlaylistFormDialog
				bind:open={editOpen}
				initial={{
					title: data.title,
					description: data.description,
					visibility: data.visibility
				}}
				onsubmit={save}
			/>
			<AlertDialog.Root bind:open={deleteOpen}>
				<AlertDialog.Content>
					<AlertDialog.Header>
						<AlertDialog.Title>{m.playlist_delete_title({ title: data.title })}</AlertDialog.Title>
						<AlertDialog.Description>{m.playlist_delete_body()}</AlertDialog.Description>
					</AlertDialog.Header>
					{#if remove.isError}
						<p class="text-sm font-medium text-destructive" role="alert">
							{m.playlist_error_save()}
						</p>
					{/if}
					<AlertDialog.Footer>
						<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
						<AlertDialog.Action
							class={buttonVariants({ variant: 'destructive' })}
							disabled={remove.isPending}
							onclick={confirmDelete}
						>
							{m.playlist_delete()}
						</AlertDialog.Action>
					</AlertDialog.Footer>
				</AlertDialog.Content>
			</AlertDialog.Root>
		{/if}
	{/if}
</div>
