<script lang="ts">
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { isAdmin } from '#lib/api/admin.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { tabLinkClass } from '#lib/components/tab-link.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	let { children }: { children: Snippet } = $props();

	const tabs = [
		{ href: resolve('/admin'), label: m.admin_users },
		{ href: resolve('/admin/status'), label: m.admin_status }
	];
	const current = $derived(page.url.pathname.replace(/\/$/, ''));
</script>

<svelte:head>
	{#if session.ready && !isAdmin()}<title>{pageTitle(m.admin_title())}</title>{/if}
</svelte:head>

<div class="page-column flex flex-col gap-5 py-6">
	<div class="flex flex-col gap-3">
		<h1 class="text-xl font-semibold tracking-tight">{m.admin_title()}</h1>
		{#if isAdmin()}
			<nav aria-label={m.admin_sections()} class="flex border-b">
				<div class="-ms-3 flex">
					{#each tabs as tab (tab.href)}
						<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved above -->
						<a
							href={tab.href}
							class={tabLinkClass}
							aria-current={current === tab.href ? 'page' : undefined}
						>
							{tab.label()}
						</a>
						<!-- eslint-enable svelte/no-navigation-without-resolve -->
					{/each}
				</div>
			</nav>
		{/if}
	</div>

	{#if !session.ready}
		<Skeleton class="h-64 w-full rounded-xl" />
	{:else if !isAdmin()}
		<EmptyState icon={ShieldIcon} title={m.admin_forbidden()} />
	{:else}
		{@render children()}
	{/if}
</div>
