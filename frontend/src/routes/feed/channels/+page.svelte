<script lang="ts">
	import ArrowDownUpIcon from '@lucide/svelte/icons/arrow-down-up';
	import UsersIcon from '@lucide/svelte/icons/users';
	import { SvelteMap } from 'svelte/reactivity';
	import { compareSubscriptions, type SubscribedChannel } from '#lib/api/subscription-list.js';
	import { createSubscriptionMutation, createSubscriptionsQuery } from '#lib/api/subscriptions.js';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import { importExportHref } from '#lib/components/subscription/links.js';
	import SubscriptionRowSkeleton from '#lib/components/subscription/subscription-row-skeleton.svelte';
	import SubscriptionRow from '#lib/components/subscription/subscription-row.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const subscriptions = createSubscriptionsQuery();
	const mutation = createSubscriptionMutation();

	// Channels unsubscribed on this page stay listed until it is left, so a slip can be undone.
	const removed = new SvelteMap<string, SubscribedChannel>();

	const rows = $derived.by(() => {
		const list = subscriptions.data ?? [];
		const ids = new Set(list.map((c) => c.id));
		const kept = [...removed.values()].filter((c) => !ids.has(c.id));
		return [...list, ...kept].sort(compareSubscriptions);
	});
	const count = $derived(subscriptions.data?.length ?? 0);

	function toggle(channel: SubscribedChannel) {
		const subscribe = removed.has(channel.id);
		if (subscribe) removed.delete(channel.id);
		else removed.set(channel.id, channel);
		mutation.mutate(
			{ channelId: channel.id, subscribe },
			{
				onError: () => {
					if (subscribe) removed.set(channel.id, channel);
					else removed.delete(channel.id);
				}
			}
		);
	}
</script>

{#snippet importExport()}
	<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
	<Button href={importExportHref()} variant="outline" size="sm" data-testid="import-export-link">
		<ArrowDownUpIcon />
		{m.subscriptions_import_export()}
	</Button>
{/snippet}

<div class="flex w-full max-w-3xl flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader
		title={m.subscriptions_title()}
		description={session.user && subscriptions.data ? m.subscriptions_count({ count }) : undefined}
		actions={session.user ? importExport : undefined}
	/>

	{#if !session.ready}
		<div class="flex flex-col" role="status" aria-label={m.subscriptions_loading()}>
			{#each { length: 6 }, i (i)}<SubscriptionRowSkeleton />{/each}
		</div>
	{:else if !session.user}
		<EmptyState
			icon={UsersIcon}
			title={m.subscriptions_title()}
			description={m.empty_sign_in_hint()}
		>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{:else if subscriptions.isPending}
		<div class="flex flex-col" role="status" aria-label={m.subscriptions_loading()}>
			{#each { length: 6 }, i (i)}<SubscriptionRowSkeleton />{/each}
		</div>
	{:else if subscriptions.isError && !subscriptions.data}
		<ErrorState onretry={() => void subscriptions.refetch()} />
	{:else if rows.length === 0}
		<EmptyState
			icon={UsersIcon}
			title={m.empty_subscriptions_title()}
			description={m.empty_subscriptions_body()}
		/>
	{:else}
		<ul class="-mx-2 flex flex-col" aria-label={m.subscriptions_title()}>
			{#each rows as channel (channel.id)}
				<li>
					<SubscriptionRow {channel} subscribed={!removed.has(channel.id)} ontoggle={toggle} />
				</li>
			{/each}
		</ul>
	{/if}
</div>
