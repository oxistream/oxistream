<script lang="ts">
	import UserRoundXIcon from '@lucide/svelte/icons/user-round-x';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import type { RoomParticipant } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		participants,
		meId,
		canKick = false,
		onkick,
		class: className
	}: {
		participants: RoomParticipant[];
		meId?: string | null;
		/** The viewer owns the room and may remove others. */
		canKick?: boolean;
		onkick?: (participant: RoomParticipant) => void;
		class?: string;
	} = $props();
</script>

<section
	aria-labelledby="room-participants-heading"
	class={cn('flex flex-col rounded-xl border bg-card', className)}
	data-testid="room-participants"
>
	<header class="flex items-center justify-between gap-2 border-b px-4 py-3">
		<h2 id="room-participants-heading" class="text-sm font-semibold">
			{m.together_participants()}
		</h2>
		<span class="text-xs text-muted-foreground tabular-nums">{participants.length}</span>
	</header>
	<ul class="flex flex-col gap-0.5 p-2">
		{#each participants as person (person.id)}
			<li class="group/person flex items-center gap-2.5 rounded-lg px-2 py-1.5 hover:bg-muted/60">
				<ChannelAvatar name={person.name} size="sm" />
				<span class="min-w-0 flex-1 truncate text-sm">
					{person.name}
					{#if person.id === meId}
						<span class="text-muted-foreground">({m.together_you()})</span>
					{/if}
				</span>
				{#if person.is_owner}
					<span
						class="rounded-full bg-primary/10 px-2 py-0.5 text-[0.6875rem] font-medium text-primary"
					>
						{m.together_host()}
					</span>
				{:else if person.is_guest}
					<span class="text-[0.6875rem] text-muted-foreground">{m.together_guest()}</span>
				{/if}
				{#if canKick && !person.is_owner && person.id !== meId}
					<Button
						variant="ghost"
						size="icon-xs"
						class="text-muted-foreground hover:text-destructive"
						aria-label={m.together_kick({ name: person.name })}
						title={m.together_kick({ name: person.name })}
						onclick={() => onkick?.(person)}
					>
						<UserRoundXIcon />
					</Button>
				{/if}
			</li>
		{/each}
	</ul>
</section>
