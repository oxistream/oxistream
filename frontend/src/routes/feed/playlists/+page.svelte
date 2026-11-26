<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_playlists()} />
	<EmptyState
		icon={ListVideoIcon}
		title={m.empty_playlists_title()}
		description={session.user ? m.empty_playlists_body() : m.empty_sign_in_hint()}
	>
		{#snippet actions()}
			{#if !session.user}<SignInActions />{/if}
		{/snippet}
	</EmptyState>
</div>
