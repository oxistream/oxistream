<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { moveItem } from '#lib/api/playlist-order.js';
	import type { PlaylistFields } from '#lib/api/playlists.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { mockPlaylist, mockPlaylistNow, mockPlaylists } from '#lib/mocks/playlists.js';
	import PlaylistCard from './playlist-card.svelte';
	import PlaylistFormDialog from './playlist-form-dialog.svelte';
	import CourseBadge from './course-badge.svelte';
	import { localPlayback, youtubePlayback } from './playback';
	import PlaylistCover from './playlist-cover.svelte';
	import PlaylistItems from './playlist-items.svelte';
	import PlaylistLayout from './playlist-layout.svelte';
	import PlaylistPanel from './playlist-panel.svelte';
	import VisibilityLabel from './visibility-label.svelte';

	const { Story } = defineMeta({
		title: 'Playlist/PlaylistCard',
		component: PlaylistCard,
		args: { playlist: mockPlaylists[0], now: mockPlaylistNow }
	});
</script>

<script lang="ts">
	let items = $state(mockPlaylist.items);
	let dialogOpen = $state(false);
	let saved = $state<PlaylistFields>();

	const youtubeList = {
		id: 'PL4lEESSgxM_5O81EvKCmBIm_JT5Q7JeaI',
		title: 'Coastal road trips',
		description: '',
		channel: { id: 'UCnorth', name: 'Northbound' },
		videoCount: 240,
		items: mockPlaylist.items
	};
</script>

<Story name="Card" />

<Story name="Card without items" args={{ playlist: mockPlaylists[2] }} />

<Story name="Overview grid" asChild>
	<div class="@container">
		<ul class="video-list" data-layout="grid">
			{#each mockPlaylists as playlist (playlist.id)}
				<li class="min-w-0"><PlaylistCard {playlist} now={mockPlaylistNow} /></li>
			{/each}
		</ul>
	</div>
</Story>

<Story name="Course badge" asChild>
	<CourseBadge />
</Story>

<Story name="Visibility labels" asChild>
	<div class="flex gap-4 text-sm text-muted-foreground">
		<VisibilityLabel visibility="private" />
		<VisibilityLabel visibility="unlisted" />
	</div>
</Story>

<Story name="Items, editable" asChild>
	<PlaylistItems
		class="max-w-3xl"
		playlistId={mockPlaylist.id}
		{items}
		editable
		onmove={(_video, from, to) => (items = moveItem(items, from, to))}
		onremove={(video) => (items = items.filter((item) => item.id !== video.id))}
	/>
</Story>

<Story name="Items, read only" asChild>
	<PlaylistItems class="max-w-3xl" playlistId={mockPlaylist.id} items={mockPlaylist.items} />
</Story>

<Story name="Watch panel" asChild>
	<PlaylistPanel
		class="h-[28rem] max-w-sm"
		playlist={localPlayback(mockPlaylist)}
		currentId={mockPlaylist.items[2].id}
	/>
</Story>

<Story name="Watch panel, YouTube playlist loading more" asChild>
	<PlaylistPanel
		class="h-[28rem] max-w-sm"
		playlist={youtubePlayback(youtubeList, true)}
		currentId={mockPlaylist.items[1].id}
		hasMore
		loadingMore
		onloadmore={() => {}}
	/>
</Story>

<Story name="Page layout" asChild>
	<PlaylistLayout>
		{#snippet hero()}
			<PlaylistCover
				src={mockPlaylist.thumbnailUrl}
				count={youtubeList.videoCount}
				class="max-w-md"
			/>
			<h1 class="text-xl font-semibold tracking-tight">{youtubeList.title}</h1>
		{/snippet}
		<PlaylistItems playlistId={youtubeList.id} items={youtubeList.items} />
	</PlaylistLayout>
</Story>

<Story name="Page layout, loading" asChild>
	<PlaylistLayout />
</Story>

<Story name="Watch panel states" asChild>
	<div class="flex max-w-sm flex-col gap-4">
		<PlaylistPanel status="loading" />
		<PlaylistPanel status="mix" />
		<PlaylistPanel status="unavailable" />
	</div>
</Story>

<Story name="Edit dialog" asChild>
	<Button onclick={() => (dialogOpen = true)}>{m.playlist_edit()}</Button>
	{#if saved}<p class="mt-2 text-sm">{saved.title} · {saved.visibility}</p>{/if}
	<PlaylistFormDialog
		bind:open={dialogOpen}
		initial={{ title: mockPlaylist.title, description: '', visibility: 'unlisted' }}
		onsubmit={(fields) => Promise.resolve((saved = fields))}
	/>
</Story>
