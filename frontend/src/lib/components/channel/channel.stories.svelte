<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockChannels } from '#lib/mocks/videos.js';
	import ChannelAvatar from './channel-avatar.svelte';
	import ChannelChip from './channel-chip.svelte';

	const { Story } = defineMeta({
		title: 'Channel/ChannelChip',
		component: ChannelChip,
		args: { channel: mockChannels.dial }
	});
</script>

<Story name="Chip" />

<Story name="Chip without image" args={{ channel: mockChannels.bindery }} />

<Story name="Chip list" asChild>
	<div class="flex max-w-md flex-wrap gap-2">
		{#each Object.values(mockChannels) as channel (channel.id)}
			<ChannelChip {channel} />
		{/each}
	</div>
</Story>

<Story name="Avatar sizes" asChild>
	<div class="flex items-center gap-3">
		<ChannelAvatar name={mockChannels.forge.name} src={mockChannels.forge.avatarUrl} size="sm" />
		<ChannelAvatar name={mockChannels.forge.name} src={mockChannels.forge.avatarUrl} />
		<ChannelAvatar name={mockChannels.forge.name} src={mockChannels.forge.avatarUrl} size="lg" />
		<ChannelAvatar name={mockChannels.bindery.name} size="sm" />
		<ChannelAvatar name={mockChannels.bindery.name} />
		<ChannelAvatar name={mockChannels.bindery.name} size="lg" />
	</div>
</Story>
