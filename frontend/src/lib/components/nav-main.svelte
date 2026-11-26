<script lang="ts" module>
	import type { Component } from 'svelte';
	import type { RouteId } from '$app/types';

	export type NavItem = {
		href: RouteId;
		label: () => string;
		icon: Component;
	};
</script>

<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { isActive } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';

	let { items }: { items: NavItem[] } = $props();
</script>

<Sidebar.Group>
	<Sidebar.GroupLabel>{m.nav_label()}</Sidebar.GroupLabel>
	<Sidebar.Menu>
		{#each items as item (item.href)}
			<Sidebar.MenuItem>
				<Sidebar.MenuButton
					isActive={isActive(page.url.pathname, item.href)}
					tooltipContent={item.label()}
				>
					{#snippet child({ props })}
						<a href={resolve(item.href)} {...props}>
							<item.icon />
							<span>{item.label()}</span>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		{/each}
	</Sidebar.Menu>
</Sidebar.Group>
