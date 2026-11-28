<script lang="ts">
	import * as Sheet from '#lib/components/ui/sheet/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { party } from '#lib/together/party.svelte.js';
	import PartyPanel from './party-panel.svelte';

	const state = $derived(party.sidebarOpen ? 'open' : 'closed');
</script>

<!-- Mirrors the app sidebar: a gap in the layout makes room, the panel itself is fixed. -->
{#if party.docked}
	<div class="group/party text-sidebar-foreground" data-state={state} data-testid="party-sidebar">
		<div
			class="w-(--party-sidebar-width) transition-[width] duration-200 ease-linear group-data-[state=closed]/party:w-0 motion-reduce:transition-none"
		></div>
		<aside
			id="party-panel"
			aria-label={m.party_title()}
			class="fixed inset-y-0 end-0 z-10 flex h-svh w-(--party-sidebar-width) p-2 transition-[right,inset-inline-end,visibility] duration-200 ease-linear group-data-[state=closed]/party:invisible group-data-[state=closed]/party:end-[calc(var(--party-sidebar-width)*-1)] motion-reduce:transition-none"
		>
			<div class="flex size-full flex-col bg-sidebar">
				<PartyPanel open={party.sidebarOpen} onclose={() => (party.panelOpen = false)} />
			</div>
		</aside>
	</div>
{:else}
	<Sheet.Root bind:open={party.sheetOpen}>
		<Sheet.Content
			side="right"
			class="gap-0 bg-sidebar p-0 text-sidebar-foreground data-[side=right]:w-[calc(100%-2rem)]"
			data-testid="party-sheet"
		>
			<PartyPanel open={party.sheetOpen} dialog onclose={() => (party.sheetOpen = false)} />
		</Sheet.Content>
	</Sheet.Root>
{/if}
