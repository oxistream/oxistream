<script lang="ts">
	import PlusIcon from '@lucide/svelte/icons/plus';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createCreateRoomMutation, createRoomsQuery } from '#lib/api/rooms.js';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import RoomCard from '#lib/components/together/room-card.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const rooms = createRoomsQuery();
	const create = createCreateRoomMutation();

	async function start() {
		try {
			const room = await create.mutateAsync(undefined);
			await goto(resolve('/together/[room]', { room: room.id }));
		} catch {
			// Shown below the header.
		}
	}
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.together_title()} description={m.together_intro()}>
		{#snippet actions()}
			{#if session.user}
				<Button size="sm" onclick={start} disabled={create.isPending}>
					<PlusIcon />
					{m.together_new()}
				</Button>
			{/if}
		{/snippet}
	</PageHeader>

	{#if create.isError}
		<p class="text-sm font-medium text-destructive" role="alert">{m.together_start_failed()}</p>
	{/if}

	{#if !session.ready}
		<VideoGrid loading skeletonCount={4} showChannel={false} />
	{:else if !session.user}
		<EmptyState icon={UsersRoundIcon} title={m.together_title()} description={m.together_sign_in()}>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{:else if rooms.isPending}
		<VideoGrid loading skeletonCount={4} showChannel={false} />
	{:else if rooms.isError}
		<ErrorState
			title={m.together_error_load()}
			description={m.error_load_body()}
			onretry={() => void rooms.refetch()}
		/>
	{:else if rooms.data.length === 0}
		<EmptyState icon={UsersRoundIcon} title={m.together_rooms_empty()}>
			{#snippet actions()}
				<Button onclick={start} disabled={create.isPending}>
					<PlusIcon />
					{m.together_new()}
				</Button>
			{/snippet}
		</EmptyState>
	{:else}
		<section class="@container flex flex-col gap-3" aria-labelledby="own-rooms">
			<h2 id="own-rooms" class="text-base font-semibold tracking-tight">
				{m.together_rooms_section()}
			</h2>
			<ul class="video-list" data-layout="grid" aria-labelledby="own-rooms">
				{#each rooms.data as room (room.id)}
					<li class="min-w-0"><RoomCard {room} /></li>
				{/each}
			</ul>
		</section>
	{/if}
</div>
