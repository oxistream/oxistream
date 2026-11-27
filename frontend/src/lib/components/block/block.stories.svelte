<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { BlockedChannel } from '#lib/api/blocks.js';
	import { mockVideos } from '#lib/mocks/videos.js';
	import { m } from '#lib/paraglide/messages.js';
	import BlockedChannelRow from './blocked-channel-row.svelte';
	import ChannelActionsMenu from './channel-actions-menu.svelte';
	import ChannelBlockedNotice from './channel-blocked-notice.svelte';

	const { Story } = defineMeta({
		title: 'Block/ChannelBlockedNotice',
		component: ChannelBlockedNotice,
		args: { channel: { name: mockVideos[0].channel.name } }
	});

	const channels: BlockedChannel[] = mockVideos.slice(0, 3).map((video, i) => ({
		id: video.channel.id,
		name: video.channel.name,
		avatarUrl: video.channel.avatarUrl,
		blockedAt: `2026-11-${20 - i}T10:00:00Z`
	}));
</script>

<script lang="ts">
	let pending = $state(false);
	let unblocked = $state<string[]>([]);
</script>

<Story name="Channel page notice" asChild>
	<div class="max-w-4xl">
		<ChannelBlockedNotice
			channel={mockVideos[0].channel}
			{pending}
			onunblock={() => (pending = !pending)}
		/>
	</div>
</Story>

<Story name="Blocked list" asChild>
	<ul class="flex max-w-3xl flex-col">
		{#each channels as channel (channel.id)}
			<li>
				<BlockedChannelRow
					{channel}
					blocked={!unblocked.includes(channel.id)}
					ontoggle={(c) =>
						(unblocked = unblocked.includes(c.id)
							? unblocked.filter((id) => id !== c.id)
							: [...unblocked, c.id])}
				/>
			</li>
		{/each}
	</ul>
</Story>

<Story name="Actions menu" asChild>
	<div class="flex items-center gap-4">
		<ChannelActionsMenu channel={mockVideos[0].channel} video={mockVideos[0]} />
		<ChannelActionsMenu
			channel={mockVideos[0].channel}
			video={mockVideos[0]}
			removeLabel={m.action_remove()}
			onremove={() => undefined}
		/>
		<div class="rounded-xl bg-neutral-900 p-3">
			<ChannelActionsMenu
				channel={mockVideos[0].channel}
				video={mockVideos[0]}
				variant="overlay"
				size="icon-lg"
			/>
		</div>
	</div>
</Story>
