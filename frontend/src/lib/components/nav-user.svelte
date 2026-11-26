<script lang="ts">
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import HistoryIcon from '@lucide/svelte/icons/history';
	import LogInIcon from '@lucide/svelte/icons/log-in';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import UserIcon from '@lucide/svelte/icons/user';
	import UserPlusIcon from '@lucide/svelte/icons/user-plus';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import * as Avatar from '#lib/components/ui/avatar/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { useSidebar } from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { initials, type User } from '#lib/video.js';

	let {
		user = null,
		onsignout = () => void session.signOut()
	}: { user?: User | null; onsignout?: () => void } = $props();

	const sidebar = useSidebar();
	const canRegister = $derived(session.capabilities?.registration_enabled !== false);
</script>

{#snippet identity()}
	<Avatar.Root class="size-8 rounded-lg">
		{#if user?.avatarUrl}
			<Avatar.Image src={user.avatarUrl} alt="" class="rounded-lg" />
		{/if}
		<Avatar.Fallback class="rounded-lg bg-sidebar-accent text-xs font-medium" aria-hidden="true">
			{#if user}{initials(user.name)}{:else}<UserIcon class="size-4" />{/if}
		</Avatar.Fallback>
	</Avatar.Root>
	<div class="grid flex-1 text-start text-sm leading-tight">
		<span class="truncate font-medium">{user ? user.name : m.user_guest()}</span>
		<span class="truncate text-xs text-muted-foreground">
			{user ? `@${user.username}` : m.user_not_signed_in()}
		</span>
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
						data-testid="user-menu"
						class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
					>
						{@render identity()}
						<span class="sr-only">{m.user_menu_label()}</span>
						<ChevronsUpDownIcon class="ms-auto size-4 text-muted-foreground" />
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
				{#if user}
					<DropdownMenu.Group>
						<DropdownMenu.Item onSelect={() => goto(resolve('/feed/history'))}>
							<HistoryIcon />
							{m.nav_history()}
						</DropdownMenu.Item>
						<DropdownMenu.Item onSelect={() => goto(resolve('/settings'))}>
							<SettingsIcon />
							{m.nav_settings()}
						</DropdownMenu.Item>
					</DropdownMenu.Group>
					<DropdownMenu.Separator />
					<DropdownMenu.Item onSelect={onsignout} data-testid="sign-out">
						<LogOutIcon />
						{m.user_sign_out()}
					</DropdownMenu.Item>
				{:else}
					<DropdownMenu.Group>
						<DropdownMenu.Item onSelect={() => goto(resolve('/login'))}>
							<LogInIcon />
							{m.user_sign_in()}
						</DropdownMenu.Item>
						{#if canRegister}
							<DropdownMenu.Item onSelect={() => goto(resolve('/register'))}>
								<UserPlusIcon />
								{m.user_register()}
							</DropdownMenu.Item>
						{/if}
					</DropdownMenu.Group>
					<DropdownMenu.Separator />
					<DropdownMenu.Item onSelect={() => goto(resolve('/settings'))}>
						<SettingsIcon />
						{m.nav_settings()}
					</DropdownMenu.Item>
				{/if}
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>
