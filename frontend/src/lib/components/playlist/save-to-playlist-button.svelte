<script lang="ts">
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';
	import SaveToPlaylistList from './save-to-playlist-list.svelte';

	let { videoId, class: className }: { videoId: string; class?: string } = $props();

	let open = $state(false);
	const base = $derived(cn('rounded-full px-3.5', className));
</script>

{#if session.user}
	<Button variant="secondary" size="sm" class={base} onclick={() => (open = true)}>
		<ListPlusIcon />
		{m.playlist_save()}
	</Button>
	<Dialog.Root bind:open>
		<Dialog.Content>
			<Dialog.Header>
				<Dialog.Title>{m.playlist_save_title()}</Dialog.Title>
			</Dialog.Header>
			<SaveToPlaylistList {videoId} />
		</Dialog.Content>
	</Dialog.Root>
{/if}
