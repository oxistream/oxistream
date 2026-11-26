<script lang="ts">
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
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
		'group/channel relative grid min-w-0 grid-cols-[auto_minmax(0,1fr)] items-center gap-4 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] @4xl:grid-cols-[22rem_minmax(0,1fr)]',
		className
	)}
>
	<div class="flex justify-center @xl:py-2">
		<ChannelAvatar
			name={channel.name}
			src={channel.avatarUrl}
			class="size-16 transition-transform duration-(--motion-image) ease-ui group-hover/channel:scale-[1.03] @xl:size-28"
			fallbackClass="text-lg @xl:text-2xl"
		/>
	</div>
	<div class="flex min-w-0 flex-col gap-1">
		<h3 class="flex min-w-0 items-center gap-1.5 text-base leading-6 font-medium">
			<a
				href={resolve('/channel/[id]', { id: channel.id })}
				data-card-link
				class="truncate outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
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
			<p class="line-clamp-2 text-xs text-pretty text-muted-foreground">{channel.description}</p>
		{/if}
	</div>
</article>
