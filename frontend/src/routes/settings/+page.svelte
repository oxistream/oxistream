<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import UserIcon from '@lucide/svelte/icons/user';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { sectionHref, visibleSections } from '#lib/settings/sections.js';

	const sections = $derived(visibleSections(session.user !== null));
</script>

<PageHeader title={m.nav_settings()} />

{#if !session.ready}
	<div class="grid gap-3 sm:grid-cols-2">
		{#each { length: 4 }, i (i)}<Skeleton class="h-20 rounded-xl" />{/each}
	</div>
{:else}
	{#if !session.user}
		<EmptyState
			icon={UserIcon}
			title={m.settings_guest_title()}
			description={m.settings_guest_body()}
		>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{/if}
	<ul class="grid gap-3 sm:grid-cols-2" aria-label={m.settings_sections()}>
		{#each sections as section (section.id)}
			<li>
				<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by sectionHref -->
				<a
					href={sectionHref(section)}
					class="group flex h-full items-center gap-4 rounded-xl border bg-card px-4 py-3.5 transition-colors duration-(--motion-in) ease-ui hover:bg-muted/60"
				>
					<span
						class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground group-hover:text-primary"
					>
						<section.icon class="size-4" />
					</span>
					<span class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="text-sm font-medium">{section.title()}</span>
						<span class="text-sm text-pretty text-muted-foreground">{section.description()}</span>
					</span>
					<ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
				</a>
				<!-- eslint-enable svelte/no-navigation-without-resolve -->
			</li>
		{/each}
	</ul>
{/if}
