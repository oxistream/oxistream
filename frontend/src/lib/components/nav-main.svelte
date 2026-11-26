<script lang="ts" module>
	import type { Component } from 'svelte';
	import type { RouteId } from '$app/types';

	export type NavItem = {
		href: Exclude<RouteId, `${string}[${string}`>;
		label: () => string;
		icon: Component;
	};
</script>

<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { isActive } from '#lib/navigation.js';
	import { cn } from '#lib/utils.js';

	let {
		items,
		label,
		class: className
	}: { items: NavItem[]; label?: string; class?: string } = $props();
</script>

<Sidebar.Group class={cn(className)}>
	{#if label}
		<Sidebar.GroupLabel>{label}</Sidebar.GroupLabel>
	{/if}
	<Sidebar.Menu>
		{#each items as item (item.href)}
			{@const active = isActive(page.url.pathname, item.href)}
			<Sidebar.MenuItem>
				<Sidebar.MenuButton isActive={active} tooltipContent={item.label()}>
					{#snippet child({ props })}
						<a
							href={resolve(item.href)}
							aria-current={active ? 'page' : undefined}
							{...props}
							class={cn(props.class as string, 'data-active:[&>svg]:text-sidebar-primary')}
						>
							<item.icon />
							<span>{item.label()}</span>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		{/each}
	</Sidebar.Menu>
</Sidebar.Group>
