<script lang="ts">
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import UserRoundXIcon from '@lucide/svelte/icons/user-round-x';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import type {
		MemberPlace,
		MemberSync,
		RoomParticipant
	} from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		participants,
		meId,
		canKick = false,
		onkick,
		onresync,
		class: className
	}: {
		participants: RoomParticipant[];
		meId?: string | null;
		/** The viewer owns the party and may remove others. */
		canKick?: boolean;
		onkick?: (participant: RoomParticipant) => void;
		/** Jumps the viewer's own player to the party; without it, there is nothing to resync. */
		onresync?: () => void;
		class?: string;
	} = $props();

	const syncs: Record<MemberSync, { label: () => string; dot: string }> = {
		synced: { label: m.party_sync_synced, dot: 'bg-emerald-500' },
		buffering: { label: m.party_sync_buffering, dot: 'bg-amber-500' },
		behind: { label: m.party_sync_behind, dot: 'bg-destructive' }
	};
	const places: Record<MemberPlace, () => string> = {
		watching: m.party_place_watching,
		browsing: m.party_place_browsing,
		shorts: m.party_place_shorts
	};
</script>

<ul class={cn('flex flex-col gap-0.5', className)} data-testid="party-members">
	{#each participants as person (person.id)}
		{@const sync = person.sync ? syncs[person.sync] : null}
		{@const place = person.place ? places[person.place]() : null}
		<li
			class="flex h-11 items-center gap-2.5 rounded-md px-2 transition-colors hover:bg-sidebar-accent"
			data-testid="party-member"
			data-name={person.name}
			data-sync={person.sync ?? ''}
			data-place={person.place ?? ''}
		>
			<span class="relative shrink-0">
				<ChannelAvatar name={person.name} size="sm" />
				{#if sync}
					<span
						class={cn(
							'absolute -end-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-sidebar',
							sync.dot
						)}
						aria-hidden="true"
					></span>
				{/if}
			</span>
			<span class="flex min-w-0 flex-1 flex-col">
				<span class="truncate text-sm leading-5">
					{person.name}
					{#if person.id === meId}
						<span class="text-muted-foreground">({m.party_you()})</span>
					{/if}
				</span>
				{#if place || sync}
					{@const detail = [place, person.sync === 'synced' ? null : sync?.label()]}
					<!-- In sync is the normal case: only the dot shows it. -->
					<span class="truncate text-xs leading-4 text-muted-foreground">
						{#if person.sync === 'synced'}<span class="sr-only">{sync?.label()}, </span>{/if}
						{detail.filter(Boolean).join(' · ')}
					</span>
				{/if}
			</span>
			{#if person.is_owner}
				<span
					class="rounded-full bg-primary/10 px-2 py-0.5 text-[0.6875rem] font-medium text-primary"
				>
					{m.party_host()}
				</span>
			{:else if person.is_guest}
				<span class="text-[0.6875rem] text-muted-foreground">{m.party_guest()}</span>
			{/if}
			{#if onresync && person.id === meId}
				<Button
					variant="ghost"
					size="icon-xs"
					class="text-muted-foreground hover:text-foreground"
					aria-label={m.party_resync()}
					title={m.party_resync()}
					onclick={onresync}
					data-testid="party-resync"
				>
					<RefreshCwIcon />
				</Button>
			{:else if canKick && !person.is_owner && person.id !== meId}
				<Button
					variant="ghost"
					size="icon-xs"
					class="text-muted-foreground hover:text-destructive"
					aria-label={m.party_kick({ name: person.name })}
					title={m.party_kick({ name: person.name })}
					onclick={() => onkick?.(person)}
				>
					<UserRoundXIcon />
				</Button>
			{/if}
		</li>
	{/each}
</ul>
