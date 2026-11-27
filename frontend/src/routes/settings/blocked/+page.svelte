<script lang="ts">
	import BanIcon from '@lucide/svelte/icons/ban';
	import { SvelteMap } from 'svelte/reactivity';
	import { createBlockMutation, createBlocksQuery, type BlockedChannel } from '#lib/api/blocks.js';
	import BlockedChannelRow from '#lib/components/block/blocked-channel-row.svelte';
	import PageHeader from '#lib/components/page-header.svelte';
	import SettingsGuestHint from '#lib/components/settings/settings-guest-hint.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import SubscriptionRowSkeleton from '#lib/components/subscription/subscription-row-skeleton.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const blocks = createBlocksQuery();
	const mutation = createBlockMutation();

	// Channels unblocked on this page stay listed until it is left, so a slip can be undone.
	const removed = new SvelteMap<string, BlockedChannel>();

	const rows = $derived.by(() => {
		const list = blocks.data ?? [];
		const ids = new Set(list.map((c) => c.id));
		const kept = [...removed.values()].filter((c) => !ids.has(c.id));
		return [...list, ...kept].sort((a, b) => b.blockedAt.localeCompare(a.blockedAt));
	});
	const count = $derived(blocks.data?.length ?? 0);

	function toggle(channel: BlockedChannel) {
		const block = removed.has(channel.id);
		if (block) removed.delete(channel.id);
		else removed.set(channel.id, channel);
		mutation.mutate(
			{ channel, block },
			{
				onError: () => {
					if (block) removed.set(channel.id, channel);
					else removed.delete(channel.id);
				}
			}
		);
	}
</script>

<PageHeader
	title={m.blocked_title()}
	description={session.user && blocks.data ? m.blocked_count({ count }) : undefined}
/>

{#if !session.ready || (session.user && blocks.isPending)}
	<div class="flex flex-col" role="status" aria-label={m.blocked_loading()}>
		{#each { length: 4 }, i (i)}<SubscriptionRowSkeleton />{/each}
	</div>
{:else if !session.user}
	<SettingsGuestHint />
{:else if blocks.isError && !blocks.data}
	<ErrorState onretry={() => void blocks.refetch()} />
{:else if rows.length === 0}
	<EmptyState icon={BanIcon} title={m.blocked_empty_title()} />
{:else}
	<ul class="-mx-2 flex flex-col" aria-label={m.blocked_title()}>
		{#each rows as channel (channel.id)}
			<li>
				<BlockedChannelRow {channel} blocked={!removed.has(channel.id)} ontoggle={toggle} />
			</li>
		{/each}
	</ul>
{/if}
