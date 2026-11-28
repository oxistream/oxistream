<script lang="ts">
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { party } from '#lib/together/party.svelte.js';

	let { videoId }: { videoId: string } = $props();
</script>

{#if party.active && party.canAdd}
	<DropdownMenu.Item
		disabled={party.status !== 'open'}
		onSelect={() => party.add(videoId)}
		data-testid="add-to-party"
	>
		<ListPlusIcon />
		{m.party_add()}
	</DropdownMenu.Item>
{/if}
