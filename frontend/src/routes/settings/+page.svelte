<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import UserIcon from '@lucide/svelte/icons/user';
	import PageHeader from '#lib/components/page-header.svelte';
	import SettingsSection from '#lib/components/settings/settings-section.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { visibleSections } from '#lib/settings/sections.js';

	const sections = $derived(visibleSections(session.user !== null));
</script>

<div class="flex w-full max-w-5xl flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_settings()} />

	<div class="grid gap-8 md:grid-cols-[11rem_minmax(0,1fr)]">
		<nav aria-label={m.settings_sections()} class="hidden md:block">
			<ul class="sticky top-20 flex flex-col gap-0.5">
				{#each sections as section (section.id)}
					<li>
						<a
							href="#{section.id}"
							class="flex h-9 items-center gap-2.5 rounded-md px-2.5 text-sm text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:bg-muted hover:text-foreground"
						>
							<section.icon class="size-4" />
							{section.title()}
						</a>
					</li>
				{/each}
			</ul>
		</nav>

		<div class="flex min-w-0 flex-col gap-6">
			{#if !session.ready}
				<Skeleton class="h-48 w-full rounded-xl" />
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
				{#each sections as section (section.id)}
					{#if section.component}
						<SettingsSection
							id={section.id}
							title={section.title()}
							description={section.description?.()}
						>
							<section.component />
						</SettingsSection>
					{:else}
						<section
							id={section.id}
							aria-labelledby="{section.id}-heading"
							class="flex scroll-mt-20 flex-col gap-3 rounded-xl border bg-card px-5 py-4 sm:flex-row sm:items-center sm:justify-between"
						>
							<div class="flex min-w-0 flex-col gap-1">
								<h2 id="{section.id}-heading" class="text-base font-semibold tracking-tight">
									{section.title()}
								</h2>
								{#if section.description}
									<p class="text-sm text-muted-foreground">{section.description()}</p>
								{/if}
							</div>
							<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved by the section -->
							<Button variant="outline" size="sm" class="w-fit shrink-0" href={section.href()}>
								{m.settings_manage()}
								<ChevronRightIcon />
							</Button>
						</section>
					{/if}
				{/each}
			{/if}
		</div>
	</div>
</div>
