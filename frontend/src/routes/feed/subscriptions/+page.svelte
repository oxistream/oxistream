<script lang="ts">
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_subscriptions()} />
	<EmptyState
		icon={TvMinimalPlayIcon}
		title={m.empty_subscriptions_title()}
		description={session.user ? m.empty_subscriptions_body() : m.empty_sign_in_hint()}
	>
		{#snippet actions()}
			{#if !session.user}<SignInActions />{/if}
		{/snippet}
	</EmptyState>
</div>
