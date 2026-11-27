<script lang="ts">
	import ListEndIcon from '@lucide/svelte/icons/list-end';
	import ListStartIcon from '@lucide/svelte/icons/list-start';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import type { QueueVideo } from '#lib/player/queue.svelte.js';
	import { party } from '#lib/together/party.svelte.js';

	let { video }: { video: QueueVideo } = $props();
</script>

<!-- In a party, its own "Add to party" takes their place. -->
{#if !party.active}
	<DropdownMenu.Item
		onSelect={() => appPlayer.enqueue(video, { next: true })}
		data-testid="queue-play-next"
	>
		<ListStartIcon />
		{m.queue_play_next()}
	</DropdownMenu.Item>
	<DropdownMenu.Item onSelect={() => appPlayer.enqueue(video)} data-testid="queue-add">
		<ListEndIcon />
		{m.queue_add()}
	</DropdownMenu.Item>
{/if}
