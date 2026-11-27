<script lang="ts">
	import BanIcon from '@lucide/svelte/icons/ban';
	import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
	import { createBlockMutation, createBlocksQuery, type BlockChange } from '#lib/api/blocks.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		channel,
		onblocked
	}: {
		channel: BlockChange['channel'];
		/** Called once the block is sent, e.g. to move on from the blocked channel's short. */
		onblocked?: () => void;
	} = $props();

	const blocks = createBlocksQuery();
	const mutation = createBlockMutation();
	const blocked = $derived(blocks.data?.some((c) => c.id === channel.id) ?? false);

	function select() {
		mutation.mutate({ channel, block: !blocked });
		if (!blocked) onblocked?.();
	}
</script>

<DropdownMenu.Item onSelect={select} data-testid="block-channel" data-blocked={blocked}>
	{#if blocked}
		<CircleCheckIcon />
		{m.block_unblock_channel()}
	{:else}
		<BanIcon />
		{m.block_channel()}
	{/if}
</DropdownMenu.Item>
