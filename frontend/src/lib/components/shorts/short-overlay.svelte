<script lang="ts">
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import ChannelActionsMenu from '#lib/components/block/channel-actions-menu.svelte';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
	import LikeButton from '#lib/components/like/like-button.svelte';
	import SubscribeButton from '#lib/components/subscription/subscribe-button.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { ChannelSummary } from '#lib/video.js';

	let {
		videoId,
		title,
		channel,
		onblocked,
		class: className
	}: {
		videoId: string;
		title: string;
		/** Without an id, the channel is shown without link and subscribe button. */
		channel?: ChannelSummary;
		/** The user just blocked the channel. */
		onblocked?: () => void;
		class?: string;
	} = $props();

	const channelHref = $derived(
		channel?.id ? resolve('/channel/[id]', { id: channel.id }) : undefined
	);
</script>

<!-- Sits over the bottom of the video and lets clicks through, except on its own controls. -->
<div
	class={cn(
		'short-overlay pointer-events-none absolute inset-x-0 bottom-0 z-10 flex items-end gap-3 bg-linear-to-t from-black/75 via-black/40 to-transparent ps-3 pe-2 pt-20 pb-3 text-white',
		className
	)}
	data-testid="short-overlay"
>
	<div class="flex min-w-0 flex-1 flex-col gap-2.5">
		{#if channel?.name || channel?.id}
			<div class="flex min-w-0 items-center gap-2">
				{#if channel.name}
					<svelte:element
						this={channelHref ? 'a' : 'div'}
						href={channelHref}
						class="pointer-events-auto flex min-w-0 items-center gap-2 rounded-full pe-1 text-sm font-semibold [text-shadow:0_1px_2px_rgb(0_0_0/40%)]"
					>
						<ChannelAvatar name={channel.name} src={channel.avatarUrl} size="sm" />
						<span class="truncate">{channel.name}</span>
						{#if channel.verified}
							<VerifiedBadge class="text-sm text-white/80" />
						{/if}
					</svelte:element>
				{/if}
				{#if channel.id}
					<SubscribeButton channelId={channel.id} size="sm" class="pointer-events-auto shrink-0" />
				{/if}
			</div>
		{/if}
		{#if title}
			<h1
				class="line-clamp-2 text-sm leading-5 font-medium text-pretty [text-shadow:0_1px_2px_rgb(0_0_0/40%)]"
			>
				{title}
			</h1>
		{/if}
	</div>
	<!-- Action column: more actions go above the watch page link. -->
	<div class="flex shrink-0 flex-col items-center gap-3" data-testid="short-actions">
		<LikeButton {videoId} variant="overlay" class="pointer-events-auto" />
		<Button
			href={resolve(`/watch?v=${videoId}`)}
			variant="ghost"
			size="icon-lg"
			class="pointer-events-auto rounded-full bg-black/35 text-white backdrop-blur-sm hover:bg-white/20 hover:text-white"
			aria-label={m.shorts_open_watch()}
			title={m.shorts_open_watch()}
			data-testid="short-watch-link"
		>
			<TvMinimalPlayIcon />
		</Button>
		{#if channel?.id}
			<ChannelActionsMenu
				{channel}
				variant="overlay"
				size="icon-lg"
				class="pointer-events-auto"
				{onblocked}
			/>
		{/if}
	</div>
</div>
