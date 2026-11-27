<script lang="ts">
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { activeRoom, addToRoom } from '#lib/together/active-room.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	let { videoId }: { videoId: string } = $props();

	const room = $derived(activeRoom());
</script>

{#if room}
	<DropdownMenu.Item onSelect={() => void addToRoom(room, videoId)} data-testid="add-to-room">
		<ListPlusIcon />
		{m.together_add_to_room({ owner: room.owner })}
	</DropdownMenu.Item>
{/if}
