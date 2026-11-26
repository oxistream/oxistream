<script lang="ts">
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import SponsorBlockPreferences from './sponsorblock-preferences.svelte';

	let { open = $bindable(false) }: { open?: boolean } = $props();
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="max-h-[min(40rem,calc(100svh-2rem))] overflow-y-auto sm:max-w-xl">
		<Dialog.Header>
			<Dialog.Title>{m.sponsorblock_title()}</Dialog.Title>
			<Dialog.Description>{m.sponsorblock_description()}</Dialog.Description>
		</Dialog.Header>
		{#if open}
			<SponsorBlockPreferences header={false} />
		{/if}
	</Dialog.Content>
</Dialog.Root>
