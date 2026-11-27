<script lang="ts" module>
	import type { ChannelDetails } from '#lib/api/channel.js';

	export type ChannelHeaderData = Omit<ChannelDetails, 'firstPage' | 'hasShorts' | 'hasLive'>;
</script>

<script lang="ts">
	import ChannelActionsMenu from '#lib/components/block/channel-actions-menu.svelte';
	import SubscribeButton from '#lib/components/subscription/subscribe-button.svelte';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';
	import ChannelAvatar from './channel-avatar.svelte';
	import VerifiedBadge from './verified-badge.svelte';

	let { channel, class: className }: { channel: ChannelHeaderData; class?: string } = $props();

	const locale = getLocale();
	const descriptionId = $props.id();
	let expanded = $state(false);
	let clamped = $state(false);
	let description = $state<HTMLElement | null>(null);

	const description_text = $derived(channel.description.trim());

	// The toggle only appears when the excerpt actually cuts text off.
	$effect(() => {
		if (!description || expanded) return;
		const element = description;
		const measure = () => (clamped = element.scrollHeight > element.clientHeight + 1);
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(element);
		return () => observer.disconnect();
	});
</script>

<header class={cn('@container flex flex-col gap-5', className)}>
	{#if channel.bannerUrl}
		<div class="aspect-[6.2/1] overflow-hidden rounded-xl bg-muted" aria-hidden="true">
			<img
				src={channel.bannerUrl}
				alt=""
				decoding="async"
				width="1707"
				height="283"
				class="size-full object-cover"
			/>
		</div>
	{/if}
	<div class="flex flex-col gap-4 @xl:flex-row @xl:items-start @xl:gap-6">
		<div class="flex min-w-0 flex-1 items-start gap-4 @xl:gap-5">
			<ChannelAvatar
				name={channel.name}
				src={channel.avatarLargeUrl ?? channel.avatarUrl}
				class="size-16 @xl:size-24"
				fallbackClass="text-xl @xl:text-3xl"
			/>
			<div class="flex min-w-0 flex-1 flex-col gap-1.5 @xl:pt-1">
				<h1
					class="flex min-w-0 items-center gap-2 text-xl font-semibold tracking-tight @xl:text-2xl"
				>
					<span class="truncate">{channel.name}</span>
					{#if channel.verified}<VerifiedBadge class="text-lg @xl:text-xl" />{/if}
				</h1>
				<p class="flex flex-wrap gap-x-1.5 text-sm text-muted-foreground tabular">
					{#if channel.handle}<span class="text-foreground">{channel.handle}</span>{/if}
					{#if channel.subscriberCount !== undefined}
						{#if channel.handle}<span aria-hidden="true">·</span>{/if}
						<span>
							{m.channel_subscribers({
								count: channel.subscriberCount,
								formatted: formatCompactNumber(channel.subscriberCount, locale)
							})}
						</span>
					{/if}
					{#if channel.videoCount !== undefined}
						<span aria-hidden="true">·</span>
						<span>
							{m.channel_video_count({
								count: channel.videoCount,
								formatted: formatCompactNumber(channel.videoCount, locale)
							})}
						</span>
					{/if}
				</p>
				{#if description_text}
					<div class="flex max-w-prose flex-col items-start gap-0.5">
						<p
							bind:this={description}
							id={descriptionId}
							class={cn(
								'text-sm text-pretty whitespace-pre-line text-muted-foreground',
								!expanded && 'line-clamp-2'
							)}
						>
							{description_text}
						</p>
						{#if clamped || expanded}
							<button
								type="button"
								class="rounded-sm text-sm font-medium text-foreground underline-offset-4 hover:underline"
								aria-expanded={expanded}
								aria-controls={descriptionId}
								onclick={() => (expanded = !expanded)}
							>
								{expanded ? m.channel_less() : m.channel_more()}
							</button>
						{/if}
					</div>
				{/if}
			</div>
		</div>
		{#if session.user}
			<div class="flex items-center gap-1.5">
				<SubscribeButton channelId={channel.id} class="flex-1 @xl:flex-none" />
				<ChannelActionsMenu {channel} size="icon" />
			</div>
		{/if}
	</div>
</header>
