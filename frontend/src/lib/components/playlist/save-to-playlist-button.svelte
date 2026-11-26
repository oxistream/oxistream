<script lang="ts">
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import { page } from '$app/state';
	import { authHref } from '#lib/auth/links.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';
	import SaveToPlaylistList from './save-to-playlist-list.svelte';

	let { videoId, class: className }: { videoId: string; class?: string } = $props();

	let open = $state(false);
	const signInHref = $derived(
		session.user ? undefined : authHref('/login', page.url.pathname + page.url.search)
	);
	const base = $derived(cn('rounded-full px-3.5', className));
</script>

{#if signInHref}
	<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved by authHref -->
	<Button
		href={signInHref}
		variant="secondary"
		size="sm"
		class={base}
		title={m.playlist_save_sign_in()}
	>
		<ListPlusIcon />
		{m.playlist_save()}
	</Button>
{:else}
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
