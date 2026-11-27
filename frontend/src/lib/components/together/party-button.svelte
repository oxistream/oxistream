<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { Button } from '#lib/components/ui/button/index.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';

	const count = $derived(party.participants.length);
	const label = $derived(
		party.active && count > 0
			? m.party_button_members({ count: formatNumber(count, getLocale()) })
			: m.party_button()
	);
</script>

<!-- Guests only see it while in a party; signed-in users start one from here. -->
{#if party.active || session.user}
	<Button
		variant="ghost"
		size="icon"
		class="relative"
		aria-label={label}
		title={label}
		aria-expanded={party.panelOpen}
		aria-controls={party.docked ? 'party-panel' : undefined}
		data-active={party.active ? '' : undefined}
		onclick={() => (party.panelOpen = !party.panelOpen)}
		data-testid="party-button"
	>
		<UsersRoundIcon class={party.active ? 'text-primary' : undefined} />
		{#if party.active && count > 0}
			<span
				class="absolute -end-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[0.625rem] leading-none font-semibold text-primary-foreground tabular-nums"
				aria-hidden="true"
				data-testid="party-count"
			>
				{formatNumber(count, getLocale())}
			</span>
		{/if}
	</Button>
{/if}
