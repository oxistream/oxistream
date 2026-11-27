<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockParticipants, mockRoomCurrent, mockRoomQueue } from '#lib/mocks/together.js';
	import { mockVideos } from '#lib/mocks/videos.js';
	import JoinForm from './join-form.svelte';
	import PartyElsewhere from './party-elsewhere.svelte';
	import PartyMembers from './party-members.svelte';
	import PartyNowPlaying from './party-now-playing.svelte';
	import PartyQueue from './party-queue.svelte';
	import PartyStatus from './party-status.svelte';
	import ShareButton from './share-button.svelte';
	import SyncReadout from './sync-readout.svelte';

	const { Story } = defineMeta({
		title: 'Together/PartyQueue',
		component: PartyQueue,
		args: { queue: mockRoomQueue }
	});
</script>

<script lang="ts">
	let queue = $state(mockRoomQueue);
	let queued = $state<'added' | null>(null);
</script>

<Story name="Queue" asChild>
	<div class="max-w-sm">
		<PartyQueue {queue} onremove={(entry) => (queue = queue.filter((e) => e.id !== entry.id))} />
	</div>
</Story>

<Story name="Queue empty" asChild>
	<div class="max-w-sm">
		<PartyQueue queue={[]} />
	</div>
</Story>

<Story name="Now playing" asChild>
	<div class="max-w-xs">
		<PartyNowPlaying current={mockRoomCurrent} playing onskip={() => {}} />
	</div>
</Story>

<Story name="Now paused" asChild>
	<div class="max-w-xs">
		<PartyNowPlaying current={mockRoomCurrent} playing={false} />
	</div>
</Story>

<Story name="Nothing playing" asChild>
	<div class="max-w-xs">
		<PartyNowPlaying current={null} playing={false} />
	</div>
</Story>

<Story name="Members as host" asChild>
	<div class="max-w-sm">
		<PartyMembers participants={mockParticipants} meId="p-mara" canKick onkick={() => {}} />
	</div>
</Story>

<Story name="Members as guest" asChild>
	<div class="max-w-sm">
		<PartyMembers participants={mockParticipants} meId="p-grace" />
	</div>
</Story>

<Story name="Connection status" asChild>
	<div class="flex flex-wrap gap-2">
		<PartyStatus status="connecting" />
		<PartyStatus status="open" />
		<PartyStatus status="reconnecting" />
	</div>
</Story>

<Story name="Join form" asChild>
	<JoinForm owner="mara" name="Grace" onjoin={() => {}} />
</Story>

<Story name="Share button" asChild>
	<ShareButton url="https://tube.example/party/Qm9vbS1vbmUtMTIzNDU2Nw" />
</Story>

<Story name="Party plays another video" asChild>
	<div class="max-w-2xl">
		<PartyElsewhere
			poster={mockVideos[1].thumbnailUrl}
			playing={mockVideos[0].title}
			{queued}
			onplaynow={() => {}}
			onqueue={() => (queued = 'added')}
		/>
	</div>
</Story>

<Story name="Party video already queued" asChild>
	<div class="max-w-2xl">
		<PartyElsewhere
			poster={mockVideos[1].thumbnailUrl}
			playing={mockVideos[0].title}
			queued="already"
			onplaynow={() => {}}
			onqueue={() => {}}
		/>
	</div>
</Story>

<Story name="Party plays nothing yet" asChild>
	<div class="max-w-2xl">
		<PartyElsewhere poster={mockVideos[1].thumbnailUrl} />
	</div>
</Story>

<Story name="Sync readout" asChild>
	<div class="relative aspect-video max-w-xl rounded-xl bg-zinc-800">
		<SyncReadout
			stats={{
				offset: -12.4,
				rtt: 3.1,
				drift: 6.8,
				rate: 1,
				lead: 0.42,
				rateCost: 0.02,
				state: 'locked'
			}}
			class="absolute start-2 top-2"
		/>
	</div>
</Story>
