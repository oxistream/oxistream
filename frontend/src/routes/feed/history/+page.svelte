<script lang="ts">
	import ClockIcon from '@lucide/svelte/icons/clock';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_history()} />
	<EmptyState
		icon={ClockIcon}
		title={m.empty_history_title()}
		description={session.user ? m.empty_history_body() : m.empty_sign_in_hint()}
	>
		{#snippet actions()}
			{#if !session.user}<SignInActions />{/if}
		{/snippet}
	</EmptyState>
</div>
