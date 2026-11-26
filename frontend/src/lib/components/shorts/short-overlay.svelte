<script lang="ts">
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
	import SubscribeButton from '#lib/components/subscription/subscribe-button.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { ChannelSummary } from '#lib/video.js';

	let {
		videoId,
		title,
		channel,
		class: className
	}: {
		videoId: string;
		title: string;
		/** Without an id, the channel is shown without link and subscribe button. */
		channel?: ChannelSummary;
		class?: string;
	} = $props();

	const channelHref = $derived(
		channel?.id ? resolve('/channel/[id]', { id: channel.id }) : undefined
	);
</script>

<!-- Sits over the top of the video and lets clicks through, except on its own controls. -->
<div
	class={cn(
		'pointer-events-none absolute inset-x-0 top-0 z-10 flex flex-col gap-2.5 bg-linear-to-b from-black/70 via-black/35 to-transparent px-3 pt-3 pb-10 text-white',
		className
	)}
	data-testid="short-overlay"
>
	<div class="flex min-w-0 items-center gap-2">
		{#if channel?.name}
			<svelte:element
				this={channelHref ? 'a' : 'div'}
				href={channelHref}
				class="pointer-events-auto flex min-w-0 items-center gap-2 rounded-full pe-1 text-sm font-semibold"
			>
				<ChannelAvatar name={channel.name} src={channel.avatarUrl} size="sm" />
				<span class="truncate">{channel.name}</span>
				{#if channel.verified}
					<VerifiedBadge class="text-sm text-white/80" />
				{/if}
			</svelte:element>
		{/if}
		{#if channel?.id}
			<SubscribeButton channelId={channel.id} size="sm" class="pointer-events-auto shrink-0" />
		{/if}
		<Button
			href={resolve(`/watch?v=${videoId}`)}
			variant="ghost"
			size="icon-sm"
			class="pointer-events-auto ms-auto shrink-0 rounded-full text-white hover:bg-white/15 hover:text-white"
			aria-label={m.shorts_open_watch()}
			title={m.shorts_open_watch()}
			data-testid="short-watch-link"
		>
			<TvMinimalPlayIcon />
		</Button>
	</div>
	{#if title}
		<h1
			class="line-clamp-2 text-sm leading-5 font-medium text-pretty [text-shadow:0_1px_2px_rgb(0_0_0/40%)]"
		>
			{title}
		</h1>
	{/if}
</div>
