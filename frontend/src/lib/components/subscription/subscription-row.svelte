<script lang="ts">
	import BellPlusIcon from '@lucide/svelte/icons/bell-plus';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import type { SubscribedChannel } from '#lib/api/subscription-list.js';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		channel,
		subscribed = true,
		ontoggle,
		actions,
		class: className
	}: {
		channel: SubscribedChannel;
		/** `false` right after unsubscribing here, so the row can offer to undo it. */
		subscribed?: boolean;
		ontoggle?: (channel: SubscribedChannel) => void;
		/** Shown before the subscribe toggle. */
		actions?: Snippet;
		class?: string;
	} = $props();

	const locale = getLocale();
	const href = $derived(resolve('/channel/[id]', { id: channel.id }));
	const label = $derived(channel.name ?? channel.handle ?? channel.id);
</script>

<div
	class={cn(
		'group/row flex min-w-0 items-center gap-3 rounded-lg px-2 py-2 transition-colors duration-(--motion-in) ease-ui hover:bg-muted/60',
		className
	)}
>
	<a
		{href}
		tabindex="-1"
		aria-hidden="true"
		class={cn('shrink-0 transition-opacity duration-(--motion-in)', !subscribed && 'opacity-50')}
	>
		{#if channel.name || channel.avatarUrl}
			<ChannelAvatar name={channel.name ?? ''} src={channel.avatarUrl} size="lg" />
		{:else}
			<span
				class="flex size-10 items-center justify-center rounded-full border border-dashed text-muted-foreground"
			>
				<LoaderCircleIcon class="size-4 animate-spin motion-reduce:animate-none" />
			</span>
		{/if}
	</a>
	<div
		class={cn(
			'flex min-w-0 flex-1 flex-col transition-opacity duration-(--motion-in)',
			!subscribed && 'opacity-60'
		)}
	>
		<a
			{href}
			class="flex max-w-full min-w-0 items-center gap-1 self-start rounded-sm text-sm font-semibold transition-colors duration-(--motion-in) ease-ui hover:text-primary"
		>
			{#if channel.name}
				<span class="truncate">{channel.name}</span>
				{#if channel.verified}<VerifiedBadge class="text-sm" />{/if}
			{:else}
				<span class="truncate font-mono text-xs font-medium text-muted-foreground">
					{channel.handle ?? channel.id}
				</span>
			{/if}
		</a>
		<p class="flex min-w-0 gap-x-1.5 truncate text-xs text-muted-foreground tabular">
			{#if !channel.name}
				<span>{m.subscriptions_channel_pending()}</span>
			{:else}
				{#if channel.handle}<span class="hidden truncate sm:inline">{channel.handle}</span>{/if}
				{#if channel.subscriberCount !== undefined}
					{#if channel.handle}<span class="hidden sm:inline" aria-hidden="true">·</span>{/if}
					<span class="shrink-0">
						{m.channel_subscribers({
							count: channel.subscriberCount,
							formatted: formatCompactNumber(channel.subscriberCount, locale)
						})}
					</span>
				{/if}
			{/if}
		</p>
	</div>
	{@render actions?.()}
	{#if ontoggle}
		{#if subscribed}
			<Button
				size="sm"
				variant="outline"
				class="shrink-0"
				aria-label={m.subscriptions_unsubscribe_from({ name: label })}
				data-testid="unsubscribe-button"
				onclick={() => ontoggle(channel)}
			>
				{m.subscriptions_unsubscribe()}
			</Button>
		{:else}
			<Button
				size="sm"
				class="shrink-0"
				aria-label={m.subscriptions_resubscribe_to({ name: label })}
				data-testid="resubscribe-button"
				onclick={() => ontoggle(channel)}
			>
				<BellPlusIcon />
				{m.subscription_subscribe()}
			</Button>
		{/if}
	{/if}
</div>
