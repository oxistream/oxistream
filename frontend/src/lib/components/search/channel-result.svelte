<script lang="ts">
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
	import SubscribeButton from '#lib/components/subscription/subscribe-button.svelte';
	import type { ChannelResult } from '#lib/api/search.js';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let { channel, class: className }: { channel: ChannelResult; class?: string } = $props();

	const locale = getLocale();
</script>

<article
	class={cn(
		'group/channel relative flex min-w-0 items-center gap-3 rounded-xl bg-muted/50 p-3 outline-offset-2 outline-ring transition-colors duration-(--motion-in) ease-ui hover:bg-muted has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:gap-4 @xl:px-4 dark:bg-muted/40 dark:hover:bg-muted/70',
		className
	)}
	data-testid="channel-result"
>
	<ChannelAvatar
		name={channel.name}
		src={channel.avatarUrl}
		class="size-12 shrink-0 @xl:size-14"
		fallbackClass="text-base"
	/>
	<div class="flex min-w-0 flex-1 flex-col gap-0.5">
		<h3 class="flex min-w-0 items-center gap-1.5 text-sm leading-5 font-medium @xl:text-base">
			<a
				href={resolve('/channel/[id]', { id: channel.id })}
				data-card-link
				class="truncate outline-none after:absolute after:inset-0 after:rounded-xl after:content-['']"
			>
				{channel.name}
			</a>
			{#if channel.verified}<VerifiedBadge class="text-sm" />{/if}
		</h3>
		<p class="truncate text-xs text-muted-foreground tabular">
			{#if channel.handle}<span>{channel.handle}</span>{/if}
			{#if channel.handle && channel.subscriberCount !== undefined}
				<span aria-hidden="true">·</span>
			{/if}
			{#if channel.subscriberCount !== undefined}
				<span>
					{m.channel_subscribers({
						count: channel.subscriberCount,
						formatted: formatCompactNumber(channel.subscriberCount, locale)
					})}
				</span>
			{/if}
		</p>
		{#if channel.description}
			<p class="hidden truncate text-xs text-muted-foreground @xl:block">{channel.description}</p>
		{/if}
	</div>
	<SubscribeButton channelId={channel.id} size="sm" class="relative z-10 shrink-0" />
</article>
