<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createCreateRoomMutation } from '#lib/api/rooms.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';

	let { videoId, class: className }: { videoId: string; class?: string } = $props();

	const create = createCreateRoomMutation();

	async function start() {
		try {
			const room = await create.mutateAsync(videoId);
			await goto(resolve('/together/[room]', { room: room.id }));
		} catch {
			// The error is shown below the button.
		}
	}
</script>

{#if session.user}
	<Button
		variant="secondary"
		size="sm"
		class={cn('rounded-full px-3.5', className)}
		title={m.together_start_hint()}
		disabled={create.isPending}
		onclick={start}
		data-testid="start-room"
	>
		<UsersRoundIcon />
		{m.together_start()}
	</Button>
	{#if create.isError}
		<p class="basis-full text-xs font-medium text-destructive" role="alert">
			{m.together_start_failed()}
		</p>
	{/if}
{/if}
