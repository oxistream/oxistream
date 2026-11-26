<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import AppHeader from './app-header.svelte';
	import AppSidebar from './app-sidebar.svelte';

	const { Story } = defineMeta({
		title: 'Shell/AppSidebar',
		component: AppSidebar,
		parameters: { layout: 'fullscreen' }
	});
</script>

<Story name="Shell" asChild>
	<Sidebar.Provider>
		<AppSidebar />
		<Sidebar.Inset>
			<AppHeader />
			<div class="flex-1 p-4"></div>
		</Sidebar.Inset>
	</Sidebar.Provider>
</Story>

<Story name="Collapsed" asChild>
	<Sidebar.Provider open={false}>
		<AppSidebar collapsible="icon" />
		<Sidebar.Inset>
			<AppHeader />
			<div class="flex-1 p-4"></div>
		</Sidebar.Inset>
	</Sidebar.Provider>
</Story>
