<script lang="ts">
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import LogInIcon from '@lucide/svelte/icons/log-in';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import UserIcon from '@lucide/svelte/icons/user';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import * as Avatar from '#lib/components/ui/avatar/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { useSidebar } from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';

	const sidebar = useSidebar();
</script>

{#snippet identity()}
	<Avatar.Root class="size-8 rounded-lg">
		<Avatar.Fallback class="rounded-lg"><UserIcon class="size-4" /></Avatar.Fallback>
	</Avatar.Root>
	<div class="grid flex-1 text-start text-sm leading-tight">
		<span class="truncate font-medium">{m.user_guest()}</span>
		<span class="truncate text-xs text-muted-foreground">{m.user_not_signed_in()}</span>
	</div>
{/snippet}

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Sidebar.MenuButton
						{...props}
						size="lg"
						class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
					>
						{@render identity()}
						<ChevronsUpDownIcon class="ms-auto size-4" />
					</Sidebar.MenuButton>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content
				class="w-(--bits-dropdown-menu-anchor-width) min-w-56 rounded-lg"
				side={sidebar.isMobile ? 'bottom' : 'right'}
				align="end"
				sideOffset={4}
			>
				<DropdownMenu.Label class="p-0 font-normal">
					<div class="flex items-center gap-2 px-1 py-1.5 text-start text-sm">
						{@render identity()}
					</div>
				</DropdownMenu.Label>
				<DropdownMenu.Separator />
				<DropdownMenu.Item onSelect={() => goto(resolve('/settings'))}>
					<SettingsIcon />
					{m.nav_settings()}
				</DropdownMenu.Item>
				<DropdownMenu.Item disabled>
					<LogInIcon />
					{m.user_sign_in()}
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>
