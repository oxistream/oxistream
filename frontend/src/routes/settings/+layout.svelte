<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import SettingsGuestHint from '#lib/components/settings/settings-guest-hint.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { SETTINGS_SECTIONS, sectionHref, visibleSections } from '#lib/settings/sections.js';
	import { cn } from '#lib/utils.js';

	let { children }: { children: Snippet } = $props();

	const overviewHref = resolve('/settings');
	const sections = $derived(visibleSections(session.user !== null));
	const overview = $derived(page.url.pathname.replace(/\/$/, '') === overviewHref);
	// Account sections show the sign-in line in place of their content.
	const accountPage = $derived(
		SETTINGS_SECTIONS.some((s) => s.account && sectionHref(s) === page.url.pathname)
	);
</script>

<div class="page-column py-6">
	<div class="grid gap-x-10 gap-y-6 md:grid-cols-[12rem_minmax(0,1fr)]">
		<nav aria-label={m.settings_sections()} class="hidden md:block">
			<div class="sticky top-20 flex flex-col gap-4">
				<h2 class="px-2.5 text-sm font-semibold tracking-tight">{m.nav_settings()}</h2>
				<ul class="flex flex-col gap-0.5">
					{#each sections as section (section.id)}
						{@const href = sectionHref(section)}
						{@const active = page.url.pathname === href}
						<li>
							<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by sectionHref -->
							<a
								{href}
								aria-current={active ? 'page' : undefined}
								class={cn(
									'flex h-9 items-center gap-2.5 rounded-md px-2.5 text-sm text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:bg-muted hover:text-foreground',
									active && 'bg-muted font-medium text-foreground [&>svg]:text-primary'
								)}
							>
								<section.icon class="size-4 shrink-0" />
								<span class="truncate">{section.title()}</span>
							</a>
							<!-- eslint-enable svelte/no-navigation-without-resolve -->
						</li>
					{/each}
				</ul>
				{#if session.ready && !session.user && !accountPage}
					<SettingsGuestHint class="border-t px-2.5 pt-4" />
				{/if}
			</div>
		</nav>

		<div class="flex max-w-3xl min-w-0 flex-col gap-4">
			{#if !overview}
				<a
					href={overviewHref}
					class="-ms-1 flex w-fit items-center gap-1 rounded-md py-1 pe-2 text-sm text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground md:hidden"
				>
					<ChevronLeftIcon class="size-4" />
					{m.settings_all()}
				</a>
			{/if}
			{@render children()}
		</div>
	</div>
</div>
