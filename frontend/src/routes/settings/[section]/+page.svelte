<script lang="ts">
	import PageHeader from '#lib/components/page-header.svelte';
	import SettingsGuestHint from '#lib/components/settings/settings-guest-hint.svelte';
	import SettingsSection from '#lib/components/settings/settings-section.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const section = $derived(data.section);
</script>

<PageHeader title={section.title()} />

{#if !session.ready}
	<Skeleton class="h-48 w-full rounded-xl" />
{:else if section.available?.() === false}
	<EmptyState icon={section.icon} title={m.error_not_found()} />
{:else if section.account && !session.user}
	<SettingsGuestHint />
{:else}
	{#key section.id}
		<SettingsSection id={section.id}>
			<section.component />
		</SettingsSection>
	{/key}
{/if}
