<script lang="ts">
	import PageHeader from '#lib/components/page-header.svelte';
	import SettingsSection from '#lib/components/settings/settings-section.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const section = $derived(data.section);
</script>

<PageHeader title={section.title()} description={section.description()} />

{#if !session.ready}
	<Skeleton class="h-48 w-full rounded-xl" />
{:else if section.account && !session.user}
	<EmptyState
		icon={section.icon}
		title={m.settings_guest_title()}
		description={m.settings_guest_body()}
	>
		{#snippet actions()}<SignInActions />{/snippet}
	</EmptyState>
{:else}
	{#key section.id}
		<SettingsSection id={section.id}>
			<section.component />
		</SettingsSection>
	{/key}
{/if}
