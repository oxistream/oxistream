<script lang="ts">
	import { QueryClientProvider } from '@tanstack/svelte-query';
	import { ModeWatcher } from 'mode-watcher';
	import { page } from '$app/state';
	import favicon from '#lib/assets/favicon.svg';
	import { createQueryClient } from '#lib/api/query-client.js';
	import AppShell from '#lib/components/app-shell.svelte';
	import AuthLayout from '#lib/components/auth/auth-layout.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, getTextDirection } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';
	import type { LayoutProps } from './$types';
	import './layout.css';

	let { children }: LayoutProps = $props();

	const queryClient = createQueryClient();

	// No server hooks in the static build, so the document language is set on the client.
	document.documentElement.lang = getLocale();
	document.documentElement.dir = getTextDirection();

	const authRoute = $derived(page.route.id === '/login' || page.route.id === '/register');
</script>

<svelte:head>
	<link rel="icon" type="image/svg+xml" href={favicon} />
	<meta name="description" content={m.app_tagline()} />
</svelte:head>

<ModeWatcher themeColors={{ light: '#f7f7f8', dark: '#0b0b0d' }} />

<a
	href="#main"
	class="fixed start-3 top-3 z-50 -translate-y-16 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground focus-visible:translate-y-0"
>
	{m.skip_to_content()}
</a>

<QueryClientProvider client={queryClient}>
	{#if page.error}
		{@render children()}
	{:else if authRoute}
		<AuthLayout>{@render children()}</AuthLayout>
	{:else}
		<AppShell user={session.user}>{@render children()}</AppShell>
	{/if}
</QueryClientProvider>
