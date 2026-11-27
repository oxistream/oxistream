<script lang="ts">
	import type { Snippet } from 'svelte';
	import AppHeader from '#lib/components/app-header.svelte';
	import { createSubscriptionsQuery } from '#lib/api/subscriptions.js';
	import AppSidebar from '#lib/components/app-sidebar.svelte';
	import QueueHost from '#lib/components/queue/queue-host.svelte';
	import PartyHost from '#lib/components/together/party-host.svelte';
	import PartySidebar from '#lib/components/together/party-sidebar.svelte';
	import AppPlayer from '#lib/components/player/app-player.svelte';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { SIDEBAR_COOKIE_NAME } from '#lib/components/ui/sidebar/constants.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';
	import type { User } from '#lib/video.js';

	let {
		user = null,
		open = $bindable(readSidebarCookie()),
		children
	}: { user?: User | null; open?: boolean; children: Snippet } = $props();

	const subscriptions = createSubscriptionsQuery();

	// Guests only have a party panel while in a party, like its button.
	const partyShown = $derived(party.active || !!session.user);
	const partyDocked = $derived(partyShown && party.docked && party.sidebarOpen);

	// The provider writes this cookie on toggle; the static build has to read it back itself.
	function readSidebarCookie() {
		return !document.cookie.split('; ').includes(`${SIDEBAR_COOKIE_NAME}=false`);
	}
</script>

<PartyHost />
<QueueHost />

<Sidebar.Provider
	bind:open
	data-party-sidebar={partyDocked ? 'open' : undefined}
	style="--party-sidebar-width: 22rem; --content-inset-end: {partyDocked
		? 'var(--party-sidebar-width)'
		: '0.5rem'};"
>
	<AppSidebar {user} channels={user ? subscriptions.data : undefined} />
	<Sidebar.Inset
		class="overflow-clip transition-[margin] duration-200 ease-linear group-data-[party-sidebar=open]/sidebar-wrapper:me-0! motion-reduce:transition-none md:ring-1 md:ring-sidebar-border"
	>
		<AppHeader />
		<main id="main" tabindex="-1" class="relative flex flex-1 flex-col outline-none">
			{#if appPlayer.media}
				<AppPlayer media={appPlayer.media} />
			{/if}
			{@render children()}
		</main>
	</Sidebar.Inset>
	{#if partyShown}<PartySidebar />{/if}
</Sidebar.Provider>
