<script lang="ts">
	import { QueryClientProvider } from '@tanstack/svelte-query';
	import { ModeWatcher } from 'mode-watcher';
	import favicon from '#lib/assets/favicon.svg';
	import { createQueryClient } from '#lib/api/query-client.js';
	import AppHeader from '#lib/components/app-header.svelte';
	import AppSidebar from '#lib/components/app-sidebar.svelte';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { getLocale, getTextDirection } from '#lib/paraglide/runtime.js';
	import type { LayoutProps } from './$types';
	import './layout.css';

	let { children }: LayoutProps = $props();

	const queryClient = createQueryClient();

	// No server hooks in the static build, so the document language is set on the client.
	document.documentElement.lang = getLocale();
	document.documentElement.dir = getTextDirection();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<ModeWatcher />

<QueryClientProvider client={queryClient}>
	<Sidebar.Provider>
		<AppSidebar />
		<Sidebar.Inset>
			<AppHeader />
			{@render children()}
		</Sidebar.Inset>
	</Sidebar.Provider>
</QueryClientProvider>
