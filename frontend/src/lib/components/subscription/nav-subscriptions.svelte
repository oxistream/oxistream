<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { SubscribedChannel } from '#lib/api/subscription-list.js';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { useSidebar } from '#lib/components/ui/sidebar/context.svelte.js';
	import { isActive } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';

	let { channels, limit = 7 }: { channels: SubscribedChannel[]; limit?: number } = $props();

	const sidebar = useSidebar();
	// The label is hidden while the sidebar shows icons only.
	const labelHidden = $derived(sidebar.state === 'collapsed' && !sidebar.isMobile);
	let expanded = $state(false);
	const shown = $derived(expanded ? channels : channels.slice(0, limit));
	const hidden = $derived(channels.length - Math.min(channels.length, limit));
	const manageHref = resolve('/feed/channels');
</script>

{#if channels.length > 0}
	<Sidebar.Group>
		<Sidebar.GroupLabel>
			{#snippet child({ props })}
				<a
					href={manageHref}
					{...props}
					class="{props.class} group/label hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
					aria-current={isActive(page.url.pathname, manageHref) ? 'page' : undefined}
					tabindex={labelHidden ? -1 : undefined}
					aria-hidden={labelHidden || undefined}
				>
					{m.nav_channels()}
					<ChevronRightIcon
						class="ms-0.5 opacity-0 transition-opacity group-hover/label:opacity-100 group-focus-visible/label:opacity-100"
					/>
				</a>
			{/snippet}
		</Sidebar.GroupLabel>
		<Sidebar.Menu>
			{#each shown as channel (channel.id)}
				{@const href = resolve('/channel/[id]', { id: channel.id })}
				{@const active = isActive(page.url.pathname, href)}
				<Sidebar.MenuItem>
					<Sidebar.MenuButton
						isActive={active}
						class="group-data-[collapsible=icon]:p-1!"
						tooltipContent={channel.name ?? m.subscriptions_channel_pending()}
					>
						{#snippet child({ props })}
							<a {href} aria-current={active ? 'page' : undefined} {...props}>
								{#if channel.name}
									<ChannelAvatar
										name={channel.name}
										src={channel.avatarUrl}
										size="sm"
										class="size-5 shrink-0 group-data-[collapsible=icon]:size-6"
										fallbackClass="text-[0.5625rem]"
									/>
									<span>{channel.name}</span>
								{:else}
									<LoaderCircleIcon
										class="animate-spin text-muted-foreground group-data-[collapsible=icon]:m-1 motion-reduce:animate-none"
									/>
									<span class="text-muted-foreground">{m.subscriptions_channel_pending()}</span>
								{/if}
							</a>
						{/snippet}
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			{/each}
			{#if hidden > 0}
				<Sidebar.MenuItem>
					<Sidebar.MenuButton
						class="text-sidebar-foreground/70"
						aria-expanded={expanded}
						tooltipContent={expanded
							? m.nav_channels_less()
							: m.nav_channels_more({ count: hidden })}
						onclick={() => (expanded = !expanded)}
					>
						{#if expanded}
							<ChevronUpIcon />
							<span>{m.nav_channels_less()}</span>
						{:else}
							<ChevronDownIcon />
							<span>{m.nav_channels_more({ count: hidden })}</span>
						{/if}
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			{/if}
		</Sidebar.Menu>
	</Sidebar.Group>
{/if}
