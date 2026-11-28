<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { createCreateRoomMutation } from '#lib/api/rooms.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';
	import { cn } from '#lib/utils.js';

	let { videoId, class: className }: { videoId: string; class?: string } = $props();

	const create = createCreateRoomMutation();

	async function start() {
		try {
			const room = await create.mutateAsync(videoId);
			party.start(room.id, videoId);
		} catch {
			// The error is shown below the button.
		}
	}
</script>

{#if session.user && !party.active}
	<Button
		variant="secondary"
		size="sm"
		class={cn('rounded-full px-3.5', className)}
		disabled={create.isPending}
		onclick={start}
		data-testid="start-party"
	>
		<UsersRoundIcon />
		{m.party_start_watch()}
	</Button>
	{#if create.isError}
		<p class="basis-full text-xs font-medium text-destructive" role="alert">
			{m.party_start_failed()}
		</p>
	{/if}
{/if}
