<script lang="ts">
	import BadgeCheckIcon from '@lucide/svelte/icons/badge-check';
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		channel,
		class: className
	}: {
		channel: {
			id: string;
			name: string;
			avatarUrl?: string;
			subscriberCount?: number | null;
			verified?: boolean;
		};
		class?: string;
	} = $props();

	const locale = getLocale();
	const href = $derived(resolve('/channel/[id]', { id: channel.id }));
</script>

<div class={cn('flex min-w-0 items-center gap-3', className)}>
	<a {href} tabindex="-1" aria-hidden="true" class="shrink-0">
		<ChannelAvatar name={channel.name} src={channel.avatarUrl} size="lg" />
	</a>
	<div class="flex min-w-0 flex-col">
		<a
			{href}
			class="flex min-w-0 items-center gap-1 rounded-sm text-sm font-semibold transition-colors duration-(--motion-in) ease-ui hover:text-primary"
		>
			<span class="truncate">{channel.name}</span>
			{#if channel.verified}
				<BadgeCheckIcon class="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
			{/if}
		</a>
		{#if channel.subscriberCount != null}
			<span class="truncate text-xs text-muted-foreground tabular">
				{m.watch_subscribers({
					count: channel.subscriberCount,
					formatted: formatCompactNumber(channel.subscriberCount, locale)
				})}
			</span>
		{/if}
	</div>
	<!-- Own provider, so the row also works outside the app shell (stories). -->
	<Tooltip.Provider>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<!-- aria-disabled instead of disabled keeps the button focusable, so the tooltip can explain why. -->
					<Button
						{...props}
						size="sm"
						variant="secondary"
						aria-disabled="true"
						data-testid="subscribe-button"
						class="ms-1 shrink-0 cursor-not-allowed rounded-full px-3.5 text-muted-foreground"
						onclick={(event: MouseEvent) => event.preventDefault()}
					>
						{m.watch_subscribe()}
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content>{m.watch_subscribe_soon()}</Tooltip.Content>
		</Tooltip.Root>
	</Tooltip.Provider>
</div>
