<script lang="ts">
	import LinkIcon from '@lucide/svelte/icons/link';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import LockIcon from '@lucide/svelte/icons/lock';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import {
		createCreatePlaylistMutation,
		createPlaylistItemMutation,
		createPlaylistsQuery
	} from '#lib/api/playlists.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { MAX_TITLE } from './playlist-form-dialog.svelte';

	let { videoId }: { videoId: string } = $props();

	const uid = $props.id();
	const playlists = createPlaylistsQuery(() => videoId);
	const item = createPlaylistItemMutation();
	const create = createCreatePlaylistMutation();

	let creating = $state(false);
	let title = $state('');
	let failed = $state(false);
	let announcement = $state('');

	const trimmed = $derived(title.trim());
	const valid = $derived(trimmed.length > 0 && trimmed.length <= MAX_TITLE);

	function toggle(playlistId: string, playlistTitle: string, add: boolean) {
		failed = false;
		item.mutate(
			{ playlistId, videoId, add },
			{
				onSuccess: () =>
					(announcement = add
						? m.playlist_saved_to({ title: playlistTitle })
						: m.playlist_removed_from({ title: playlistTitle })),
				onError: () => (failed = true)
			}
		);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!valid) return;
		failed = false;
		try {
			const playlist = await create.mutateAsync({
				title: trimmed,
				description: '',
				visibility: 'private'
			});
			title = '';
			creating = false;
			toggle(playlist.id, playlist.title, true);
		} catch {
			failed = true;
		}
	}
</script>

<div class="flex flex-col gap-4">
	{#if playlists.isPending}
		<div class="flex flex-col gap-3" role="status" aria-label={m.playlist_loading()}>
			{#each { length: 3 }, i (i)}
				<Skeleton class="h-5 w-2/3" />
			{/each}
		</div>
	{:else if playlists.isError}
		<p class="text-sm text-destructive" role="alert">{m.playlist_error_load()}</p>
	{:else if playlists.data.length === 0 && !creating}
		<p class="text-sm text-muted-foreground">{m.empty_playlists_title()}</p>
	{:else}
		<ul class="-mx-2 flex max-h-72 flex-col overflow-y-auto" aria-label={m.playlist_save_list()}>
			{#each playlists.data as playlist (playlist.id)}
				{@const id = `${uid}-${playlist.id}`}
				<li>
					<div
						class="flex min-h-11 items-center gap-3 rounded-md px-2 transition-colors duration-(--motion-in) ease-ui hover:bg-muted"
					>
						<Checkbox
							{id}
							checked={playlist.containsVideo === true}
							onCheckedChange={(checked) => toggle(playlist.id, playlist.title, checked)}
						/>
						<Label for={id} class="min-w-0 flex-1 cursor-pointer py-2 font-normal">
							<span class="truncate">{playlist.title}</span>
						</Label>
						{#if playlist.visibility === 'unlisted'}
							<LinkIcon
								class="size-4 shrink-0 text-muted-foreground"
								aria-label={m.playlist_visibility_unlisted()}
							/>
						{:else}
							<LockIcon
								class="size-4 shrink-0 text-muted-foreground"
								aria-label={m.playlist_visibility_private()}
							/>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}

	{#if failed}
		<p class="text-sm font-medium text-destructive" role="alert">{m.playlist_error_save()}</p>
	{/if}
	<p class="sr-only" aria-live="polite">{announcement}</p>

	{#if creating}
		<form class="flex flex-col gap-2" onsubmit={submit} novalidate>
			<Label for="{uid}-new">{m.playlist_field_title()}</Label>
			<div class="flex gap-2">
				<Input
					id="{uid}-new"
					bind:value={title}
					autocomplete="off"
					autofocus
					maxlength={MAX_TITLE}
					class="h-10 text-base md:text-sm"
				/>
				<Button type="submit" class="h-10" disabled={!valid || create.isPending}>
					{#if create.isPending}
						<LoaderCircleIcon class="animate-spin motion-reduce:animate-none" />
					{/if}
					{m.playlist_create_submit()}
				</Button>
			</div>
		</form>
	{:else if playlists.isSuccess}
		<Button variant="outline" class="w-fit" onclick={() => (creating = true)}>
			<PlusIcon />
			{m.playlist_new()}
		</Button>
	{/if}
</div>
