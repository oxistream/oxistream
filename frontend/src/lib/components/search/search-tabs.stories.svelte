<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import SearchXIcon from '@lucide/svelte/icons/search-x';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { mockChannels } from '#lib/mocks/videos.js';
	import { m } from '#lib/paraglide/messages.js';
	import ChannelResult from './channel-result.svelte';
	import SearchTabs from './search-tabs.svelte';

	const { Story } = defineMeta({
		title: 'Search/SearchTabs',
		component: SearchTabs,
		args: { query: 'forge', current: 'all' }
	});

	const channels = [
		{
			...mockChannels.forge,
			verified: true,
			subscriberCount: 284_000,
			description: 'Traditional blacksmithing from a one-person forge.'
		},
		{ ...mockChannels.bindery, verified: false, description: '' }
	];
</script>

<Story name="All" />

<Story name="Shorts" args={{ current: 'shorts' }} />

<Story name="Narrow" asChild>
	<div class="max-w-[375px] p-4"><SearchTabs query="forge" current="playlists" /></div>
</Story>

<Story name="Channel rows" asChild>
	<div class="max-w-5xl p-6">
		<SearchTabs query="forge" current="channels" />
		<ul class="@container mt-5 flex flex-col gap-6">
			{#each channels as channel (channel.id)}
				<li><ChannelResult {channel} /></li>
			{/each}
		</ul>
	</div>
</Story>

<Story name="Channel rows narrow" asChild>
	<ul class="@container flex max-w-[375px] flex-col gap-6 p-4">
		{#each channels as channel (channel.id)}
			<li><ChannelResult {channel} /></li>
		{/each}
	</ul>
</Story>

<Story name="No shorts" asChild>
	<div class="max-w-5xl p-6">
		<EmptyState
			icon={SearchXIcon}
			title={m.search_empty_shorts({ query: 'qwxz forge' })}
			description={m.search_empty_body()}
		/>
	</div>
</Story>
