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
	import { isLocalPlaylistId, resumeVideo } from '#lib/api/playlist-order.js';
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
	import PlaylistCover from '#lib/components/playlist/playlist-cover.svelte';
	import PlaylistFormDialog from '#lib/components/playlist/playlist-form-dialog.svelte';
	import PlaylistItems from '#lib/components/playlist/playlist-items.svelte';
	import VisibilityLabel from '#lib/components/playlist/visibility-label.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { formatDurationLong, formatNumber } from '#lib/format.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	const locale = getLocale();
	const client = useQueryClient();
	const listId = $derived(page.url.searchParams.get('list'));
	const supported = $derived(isLocalPlaylistId(listId));
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
			: pageTitle(
					!supported && listId
						? m.playlist_unsupported_title()
						: notFound
							? m.playlist_not_found_title()
							: m.search_playlist()
				)
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
	<title>{title}</title>
</svelte:head>

<div class="px-(--page-gutter) py-6">
	{#if !data}<h1 class="sr-only">{m.search_playlist()}</h1>{/if}
	{#if !listId}
		<EmptyState
			icon={ListVideoIcon}
			title={m.playlist_none_title()}
			description={m.playlist_none_body()}
		/>
	{:else if !supported}
		<EmptyState
			icon={ListVideoIcon}
			title={m.playlist_unsupported_title()}
			description={m.playlist_unsupported_body()}
		/>
	{:else if playlist.isPending}
		<div class="playlist-layout" role="status" aria-label={m.playlist_loading()}>
			<div class="flex flex-col gap-3">
				<Skeleton class="aspect-video w-full rounded-lg" />
				<Skeleton class="h-6 w-3/4" />
				<Skeleton class="h-4 w-1/2" />
			</div>
			<div class="flex flex-col gap-3">
				{#each { length: 6 }, i (i)}
					<div class="flex items-center gap-3">
						<Skeleton class="aspect-video w-32 shrink-0 rounded-md sm:w-40" />
						<Skeleton class="h-4 flex-1" />
					</div>
				{/each}
			</div>
		</div>
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
		<div class="playlist-layout">
			<aside class="playlist-hero flex flex-col gap-4">
				<PlaylistCover src={data.thumbnailUrl} count={data.items.length} class="max-w-md" />
				<div class="flex min-w-0 flex-col gap-1.5">
					<h1 class="text-xl font-semibold tracking-tight text-pretty break-words">
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
				<div class="flex flex-wrap gap-2">
					{#if continuing && resume}
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
						<Button href={playlistWatchHref(resume.id, data.id)} data-testid="playlist-continue">
							<PlayIcon class="fill-current" />
							{m.playlist_continue()}
						</Button>
					{/if}
					{#if start}
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
						<Button
							href={playlistWatchHref(start.id, data.id)}
							variant={continuing ? 'outline' : 'default'}
						>
							{#if continuing}<RotateCcwIcon />{:else}<PlayIcon class="fill-current" />{/if}
							{continuing ? m.playlist_play_from_start() : m.playlist_play_all()}
						</Button>
					{/if}
					{#if data.visibility === 'unlisted'}
						<Button variant="outline" onclick={share}>
							{#if copied}<CheckIcon />{:else}<Share2Icon />{/if}
							{copied ? m.playlist_share_copied() : m.playlist_share()}
						</Button>
					{/if}
					{#if data.isOwner}
						<span class="flex gap-2">
							<Button
								variant="outline"
								size="icon"
								onclick={() => (editOpen = true)}
								aria-label={m.playlist_edit()}
								title={m.playlist_edit()}
							>
								<PencilIcon />
							</Button>
							<Button
								variant="outline"
								size="icon"
								onclick={() => (deleteOpen = true)}
								aria-label={m.playlist_delete()}
								title={m.playlist_delete()}
							>
								<Trash2Icon />
							</Button>
						</span>
					{/if}
				</div>
				<p class="sr-only" aria-live="polite">{copied ? m.playlist_share_copied() : ''}</p>
				{#if data.isOwner && data.visibility === 'private'}
					<p class="text-xs text-muted-foreground">{m.playlist_private_hint()}</p>
				{/if}
			</aside>

			<section aria-label={m.playlist_items_label()} class="min-w-0">
				{#if data.items.length === 0}
					<EmptyState
						icon={ListVideoIcon}
						title={m.playlist_empty_title()}
						description={m.playlist_empty_body()}
						headingLevel={2}
					/>
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
			</section>
		</div>

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

<style>
	.playlist-layout {
		display: grid;
		gap: 1.5rem;
		max-width: 96rem;
		margin-inline: auto;
	}

	@media (width >= 64rem) {
		.playlist-layout {
			grid-template-columns: 24rem minmax(0, 1fr);
			gap: 2rem;
			align-items: start;
		}

		.playlist-hero {
			position: sticky;
			top: 5rem;
		}
	}
</style>
