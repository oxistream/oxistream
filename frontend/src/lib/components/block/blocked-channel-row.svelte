<script lang="ts">
	import BanIcon from '@lucide/svelte/icons/ban';
	import { resolve } from '$app/paths';
	import type { BlockedChannel } from '#lib/api/blocks.js';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { formatDate } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		channel,
		blocked = true,
		ontoggle,
		class: className
	}: {
		channel: BlockedChannel;
		/** `false` right after unblocking here, so the row can offer to undo it. */
		blocked?: boolean;
		ontoggle?: (channel: BlockedChannel) => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	const href = $derived(resolve('/channel/[id]', { id: channel.id }));
</script>

<div
	class={cn(
		'flex min-w-0 items-center gap-3 rounded-lg px-2 py-2 transition-colors duration-(--motion-in) ease-ui hover:bg-muted/60',
		className
	)}
	data-testid="blocked-channel"
>
	<a
		{href}
		tabindex="-1"
		aria-hidden="true"
		class={cn('shrink-0 transition duration-(--motion-in)', blocked && 'opacity-60 grayscale')}
	>
		<ChannelAvatar name={channel.name} src={channel.avatarUrl} size="lg" />
	</a>
	<div class="flex min-w-0 flex-1 flex-col">
		<a
			{href}
			class="self-start truncate rounded-sm text-sm font-semibold transition-colors duration-(--motion-in) ease-ui hover:text-primary"
		>
			{channel.name}
		</a>
		<p class="truncate text-xs text-muted-foreground tabular">
			{m.blocked_since({ date: formatDate(channel.blockedAt, locale) })}
		</p>
	</div>
	{#if ontoggle}
		{#if blocked}
			<Button
				size="sm"
				variant="outline"
				class="shrink-0"
				aria-label={m.block_unblock_named({ name: channel.name })}
				data-testid="unblock-button"
				onclick={() => ontoggle(channel)}
			>
				{m.block_unblock()}
			</Button>
		{:else}
			<Button
				size="sm"
				variant="secondary"
				class="shrink-0"
				data-testid="reblock-button"
				onclick={() => ontoggle(channel)}
			>
				<BanIcon />
				{m.block_channel()}
			</Button>
		{/if}
	{/if}
</div>
