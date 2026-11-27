<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import { Button } from '#lib/components/ui/button/index.js';
	import { activeRoom, addToRoom, type ActiveRoom } from '#lib/together/active-room.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	const FEEDBACK_MS = 4000;

	let {
		videoId,
		room: fixedRoom,
		class: className
	}: {
		videoId: string;
		/** Defaults to the room open in another tab; set for stories. */
		room?: ActiveRoom | null;
		class?: string;
	} = $props();

	const room = $derived(fixedRoom === undefined ? activeRoom() : fixedRoom);
	let result = $state<'added' | 'failed' | null>(null);
	let pending = $state(false);

	$effect(() => {
		if (!result) return;
		const timer = setTimeout(() => (result = null), FEEDBACK_MS);
		return () => clearTimeout(timer);
	});

	// A different video starts over.
	$effect(() => {
		void videoId;
		result = null;
	});

	async function add(target: ActiveRoom) {
		pending = true;
		result = (await addToRoom(target, videoId)) ? 'added' : 'failed';
		pending = false;
	}
</script>

{#if room}
	<Button
		variant="secondary"
		size="sm"
		class={cn('rounded-full px-3.5', className)}
		disabled={pending || result === 'added'}
		onclick={() => add(room)}
		data-testid="add-to-room"
	>
		{#if result === 'added'}
			<CheckIcon />
			{m.together_added_to_room({ owner: room.owner })}
		{:else}
			<ListPlusIcon />
			{m.together_add_to_room({ owner: room.owner })}
		{/if}
	</Button>
	{#if result === 'failed'}
		<p class="basis-full text-xs font-medium text-destructive" role="alert">
			{m.together_add_to_room_failed()}
		</p>
	{/if}
{/if}
