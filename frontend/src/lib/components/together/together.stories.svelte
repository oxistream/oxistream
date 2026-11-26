<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import {
		mockParticipants,
		mockRoomCurrent,
		mockRoomNow,
		mockRoomQueue,
		mockRooms
	} from '#lib/mocks/together.js';
	import JoinForm from './join-form.svelte';
	import RoomAdd from './room-add.svelte';
	import RoomCard from './room-card.svelte';
	import RoomParticipants from './room-participants.svelte';
	import RoomQueue from './room-queue.svelte';
	import RoomStatus from './room-status.svelte';
	import ShareButton from './share-button.svelte';

	const { Story } = defineMeta({
		title: 'Together/RoomQueue',
		component: RoomQueue,
		args: { current: mockRoomCurrent, queue: mockRoomQueue }
	});
</script>

<script lang="ts">
	let queue = $state(mockRoomQueue);
	let added = $state<string[]>([]);
</script>

<Story name="Queue" asChild>
	<div class="max-w-sm">
		<RoomQueue
			current={mockRoomCurrent}
			{queue}
			onremove={(entry) => (queue = queue.filter((e) => e.id !== entry.id))}
		>
			{#snippet add()}<RoomAdd onadd={(id) => (added = [...added, id])} />{/snippet}
		</RoomQueue>
		{#if added.length}
			<p class="mt-2 text-xs text-muted-foreground">Added: {added.join(', ')}</p>
		{/if}
	</div>
</Story>

<Story name="Queue empty" asChild>
	<div class="max-w-sm">
		<RoomQueue current={null} queue={[]}>
			{#snippet add()}<RoomAdd onadd={() => {}} error="The queue is full." />{/snippet}
		</RoomQueue>
	</div>
</Story>

<Story name="Participants as host" asChild>
	<div class="max-w-sm">
		<RoomParticipants participants={mockParticipants} meId="p-mara" canKick onkick={() => {}} />
	</div>
</Story>

<Story name="Participants as guest" asChild>
	<div class="max-w-sm">
		<RoomParticipants participants={mockParticipants} meId="p-grace" />
	</div>
</Story>

<Story name="Connection status" asChild>
	<div class="flex flex-wrap gap-2">
		<RoomStatus status="connecting" />
		<RoomStatus status="open" />
		<RoomStatus status="reconnecting" />
	</div>
</Story>

<Story name="Room cards" asChild>
	<div class="@container max-w-3xl">
		<ul class="video-list" data-layout="grid">
			{#each mockRooms as room (room.id)}
				<li class="min-w-0"><RoomCard {room} now={mockRoomNow} /></li>
			{/each}
		</ul>
	</div>
</Story>

<Story name="Join form" asChild>
	<JoinForm owner="mara" name="Grace" onjoin={() => {}} />
</Story>

<Story name="Share button" asChild>
	<ShareButton url="https://tube.example/together/Qm9vbS1vbmUtMTIzNDU2Nw" />
</Story>
