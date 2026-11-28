<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { SubscribedChannel } from '#lib/api/subscription-list.js';
	import { mockPendingSubscription, mockSubscriptions } from '#lib/mocks/videos.js';
	import GroupTabs from './group-tabs.svelte';
	import SubscribeToggle from './subscribe-toggle.svelte';
	import SubscriptionRowSkeleton from './subscription-row-skeleton.svelte';
	import SubscriptionRow from './subscription-row.svelte';

	const { Story } = defineMeta({
		title: 'Subscription/SubscribeToggle',
		component: SubscribeToggle,
		args: { subscribed: false }
	});
</script>

<script lang="ts">
	let subscribed = $state(false);
	const groups = ['Tech', 'Music', 'Woodworking & Crafts', 'News'].map((name, i) => ({
		id: `group-${i}`,
		name,
		showOnHome: false,
		channelIds: []
	}));
	let removed = $state<string[]>([]);

	function toggle(channel: SubscribedChannel) {
		removed = removed.includes(channel.id)
			? removed.filter((id) => id !== channel.id)
			: [...removed, channel.id];
	}
</script>

<Story name="Not subscribed" />

<Story name="Subscribed" args={{ subscribed: true }} />

<Story name="Loading" args={{ loading: true }} />

<Story name="Interactive" asChild>
	<div class="flex items-center gap-3">
		<SubscribeToggle {subscribed} onclick={() => (subscribed = !subscribed)} />
		<SubscribeToggle {subscribed} size="sm" onclick={() => (subscribed = !subscribed)} />
	</div>
</Story>

<Story name="Channel list" asChild>
	<ul class="flex max-w-2xl flex-col">
		{#each [...mockSubscriptions, mockPendingSubscription] as channel (channel.id)}
			<li>
				<SubscriptionRow {channel} subscribed={!removed.includes(channel.id)} ontoggle={toggle} />
			</li>
		{/each}
	</ul>
</Story>

<Story name="Channel list skeleton" asChild>
	<div class="flex max-w-2xl flex-col">
		{#each { length: 4 }, i (i)}<SubscriptionRowSkeleton />{/each}
	</div>
</Story>

<Story name="Channel list at phone width" asChild>
	<ul class="flex max-w-[390px] flex-col">
		{#each mockSubscriptions.slice(0, 4) as channel (channel.id)}
			<li>
				<SubscriptionRow {channel} ontoggle={toggle} subscribed={!removed.includes(channel.id)} />
			</li>
		{/each}
	</ul>
</Story>

<Story name="Group tabs" asChild>
	<GroupTabs {groups} group="group-1" href={(id) => `#${id ?? ''}`} class="max-w-md" />
</Story>
