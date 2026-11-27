<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import PageHeader from '#lib/components/page-header.svelte';
	import SettingsGuestHint from '#lib/components/settings/settings-guest-hint.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { sectionHref, visibleSections } from '#lib/settings/sections.js';

	const sections = $derived(visibleSections(session.user !== null));
</script>

<PageHeader title={m.nav_settings()} />

{#if !session.ready}
	<Skeleton class="h-80 rounded-xl" />
{:else}
	<ul
		class="flex flex-col divide-y overflow-hidden rounded-xl border bg-card"
		aria-label={m.settings_sections()}
	>
		{#each sections as section (section.id)}
			<li>
				<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by sectionHref -->
				<a
					href={sectionHref(section)}
					class="flex h-12 items-center gap-3 px-4 text-sm font-medium transition-colors duration-(--motion-in) ease-ui hover:bg-muted/60"
				>
					<section.icon class="size-4 shrink-0 text-muted-foreground" />
					<span class="min-w-0 flex-1 truncate">{section.title()}</span>
					<ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
				</a>
				<!-- eslint-enable svelte/no-navigation-without-resolve -->
			</li>
		{/each}
	</ul>
	{#if !session.user}
		<SettingsGuestHint class="px-1" />
	{/if}
{/if}
