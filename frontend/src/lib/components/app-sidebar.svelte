<script lang="ts">
	import ClockIcon from '@lucide/svelte/icons/clock';
	import HouseIcon from '@lucide/svelte/icons/house';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import type { ComponentProps } from 'svelte';
	import { afterNavigate } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { SubscribedChannel } from '#lib/api/subscription-list.js';
	import LogoMark from '#lib/components/brand/logo-mark.svelte';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { useSidebar } from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { User } from '#lib/video.js';
	import NavMain, { type NavItem } from './nav-main.svelte';
	import NavSettings from './nav-settings.svelte';
	import NavUser from './nav-user.svelte';
	import NavSubscriptions from './subscription/nav-subscriptions.svelte';

	let {
		ref = $bindable(null),
		user = null,
		channels = [],
		onsignout,
		...restProps
	}: ComponentProps<typeof Sidebar.Root> & {
		user?: User | null;
		/** The signed-in user's subscriptions, listed compactly below the library. */
		channels?: SubscribedChannel[];
		onsignout?: () => void;
	} = $props();

	const sidebar = useSidebar();

	afterNavigate(() => sidebar.setOpenMobile(false));

	const primary: NavItem[] = [
		{ href: '/', label: m.nav_home, icon: HouseIcon },
		{ href: '/feed/subscriptions', label: m.nav_subscriptions, icon: TvMinimalPlayIcon }
	];
	const library: NavItem[] = [
		{ href: '/feed/playlists', label: m.nav_playlists, icon: ListVideoIcon },
		{ href: '/feed/history', label: m.nav_history, icon: ClockIcon }
	];
</script>

<Sidebar.Root bind:ref variant="inset" collapsible="icon" {...restProps}>
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton size="lg" tooltipContent="Oxistream">
					{#snippet child({ props })}
						<a href={resolve('/')} {...props}>
							<span class="flex size-8 shrink-0 items-center justify-center">
								<LogoMark class="size-7! text-sidebar-primary" />
							</span>
							<span class="grid flex-1 text-start leading-tight">
								<span class="truncate text-[1.0625rem] font-semibold tracking-[-0.02em]">
									Oxistream
								</span>
								<span class="truncate text-xs text-muted-foreground">{m.app_tagline()}</span>
							</span>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		<nav aria-label={m.nav_label()} class="contents">
			<NavMain items={primary} />
			<NavMain items={library} label={m.nav_library()} />
			{#if user}<NavSubscriptions {channels} />{/if}
			<NavSettings signedIn={user !== null} class="mt-auto" />
		</nav>
	</Sidebar.Content>
	<Sidebar.Footer>
		<NavUser {user} {onsignout} />
	</Sidebar.Footer>
</Sidebar.Root>
