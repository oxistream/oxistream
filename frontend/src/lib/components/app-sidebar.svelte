<script lang="ts">
	import ClockIcon from '@lucide/svelte/icons/clock';
	import HouseIcon from '@lucide/svelte/icons/house';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PlayIcon from '@lucide/svelte/icons/play';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import type { ComponentProps } from 'svelte';
	import { resolve } from '$app/paths';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import NavMain, { type NavItem } from './nav-main.svelte';
	import NavUser from './nav-user.svelte';

	let { ref = $bindable(null), ...restProps }: ComponentProps<typeof Sidebar.Root> = $props();

	const items: NavItem[] = [
		{ href: '/', label: m.nav_home, icon: HouseIcon },
		{ href: '/feed/subscriptions', label: m.nav_subscriptions, icon: TvMinimalPlayIcon },
		{ href: '/feed/playlists', label: m.nav_playlists, icon: ListVideoIcon },
		{ href: '/feed/history', label: m.nav_history, icon: ClockIcon },
		{ href: '/settings', label: m.nav_settings, icon: SettingsIcon }
	];
</script>

<Sidebar.Root bind:ref variant="inset" {...restProps}>
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton size="lg">
					{#snippet child({ props })}
						<a href={resolve('/')} {...props}>
							<div
								class="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground"
							>
								<PlayIcon class="size-4" />
							</div>
							<div class="grid flex-1 text-start text-sm leading-tight">
								<span class="truncate font-medium">Oxistream</span>
								<span class="truncate text-xs text-muted-foreground">{m.app_tagline()}</span>
							</div>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		<NavMain {items} />
	</Sidebar.Content>
	<Sidebar.Footer>
		<NavUser />
	</Sidebar.Footer>
</Sidebar.Root>
