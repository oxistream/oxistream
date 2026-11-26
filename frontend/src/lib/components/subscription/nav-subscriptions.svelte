<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { SubscribedChannel } from '#lib/api/subscription-list.js';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { isActive } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';

	let { channels, limit = 7 }: { channels: SubscribedChannel[]; limit?: number } = $props();

	const shown = $derived(channels.slice(0, limit));
	const manageHref = resolve('/feed/channels');
</script>

{#if channels.length > 0}
	<Sidebar.Group>
		<Sidebar.GroupLabel>{m.nav_channels()}</Sidebar.GroupLabel>
		<Sidebar.Menu>
			{#each shown as channel (channel.id)}
				{@const href = resolve('/channel/[id]', { id: channel.id })}
				{@const active = isActive(page.url.pathname, href)}
				<Sidebar.MenuItem>
					<Sidebar.MenuButton
						isActive={active}
						tooltipContent={channel.name ?? m.subscriptions_channel_pending()}
					>
						{#snippet child({ props })}
							<a {href} aria-current={active ? 'page' : undefined} {...props}>
								{#if channel.name}
									<ChannelAvatar
										name={channel.name}
										src={channel.avatarUrl}
										size="sm"
										class="size-5 shrink-0 group-data-[collapsible=icon]:size-4"
										fallbackClass="text-[0.5625rem]"
									/>
									<span>{channel.name}</span>
								{:else}
									<LoaderCircleIcon
										class="animate-spin text-muted-foreground motion-reduce:animate-none"
									/>
									<span class="text-muted-foreground">{m.subscriptions_channel_pending()}</span>
								{/if}
							</a>
						{/snippet}
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			{/each}
			<Sidebar.MenuItem>
				<Sidebar.MenuButton
					isActive={isActive(page.url.pathname, manageHref)}
					class="text-sidebar-foreground/70"
					tooltipContent={channels.length > shown.length
						? m.nav_channels_all({ count: channels.length })
						: m.nav_channels_manage()}
				>
					{#snippet child({ props })}
						<a href={manageHref} {...props}>
							<ChevronRightIcon />
							<span>
								{channels.length > shown.length
									? m.nav_channels_all({ count: channels.length })
									: m.nav_channels_manage()}
							</span>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Group>
{/if}
