<script lang="ts">
	import type { Snippet } from 'svelte';
	import AppHeader from '#lib/components/app-header.svelte';
	import { createSubscriptionsQuery } from '#lib/api/subscriptions.js';
	import AppSidebar from '#lib/components/app-sidebar.svelte';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { SIDEBAR_COOKIE_NAME } from '#lib/components/ui/sidebar/constants.js';
	import type { User } from '#lib/video.js';

	let {
		user = null,
		open = $bindable(readSidebarCookie()),
		children
	}: { user?: User | null; open?: boolean; children: Snippet } = $props();

	const subscriptions = createSubscriptionsQuery();

	// The provider writes this cookie on toggle; the static build has to read it back itself.
	function readSidebarCookie() {
		return !document.cookie.split('; ').includes(`${SIDEBAR_COOKIE_NAME}=false`);
	}
</script>

<Sidebar.Provider bind:open>
	<AppSidebar {user} channels={user ? subscriptions.data : undefined} />
	<Sidebar.Inset class="overflow-clip md:ring-1 md:ring-sidebar-border">
		<AppHeader />
		<main id="main" tabindex="-1" class="flex flex-1 flex-col outline-none">
			{@render children()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
