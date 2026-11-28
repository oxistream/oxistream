<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import SearchIcon from '@lucide/svelte/icons/search';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import ShieldOffIcon from '@lucide/svelte/icons/shield-off';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import UserCheckIcon from '@lucide/svelte/icons/user-check';
	import UserXIcon from '@lucide/svelte/icons/user-x';
	import UsersIcon from '@lucide/svelte/icons/users';
	import { toast } from 'svelte-sonner';
	import {
		ADMIN_PAGE_SIZE,
		createAdminUserMutation,
		createAdminUsersQuery,
		type AdminAction,
		type AdminUser
	} from '#lib/api/admin.js';
	import { ApiError } from '#lib/api/mapping.js';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import { formatDate, formatNumber, formatRelativeTime } from '#lib/format.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	const SEARCH_DEBOUNCE_MS = 300;

	let input = $state('');
	let query = $state('');
	let pageIndex = $state(0);

	$effect(() => {
		const value = input.trim();
		if (value === query) return;
		const timer = setTimeout(() => {
			query = value;
			pageIndex = 0;
		}, SEARCH_DEBOUNCE_MS);
		return () => clearTimeout(timer);
	});

	const users = createAdminUsersQuery(() => ({ q: query, page: pageIndex }));
	const mutation = createAdminUserMutation();

	const rows = $derived(users.data?.users ?? []);
	const total = $derived(users.data?.total ?? 0);
	const from = $derived(pageIndex * ADMIN_PAGE_SIZE + 1);
	const to = $derived(Math.min(total, (pageIndex + 1) * ADMIN_PAGE_SIZE));
	const locale = $derived(getLocale());

	let confirming = $state<{ kind: 'disable' | 'delete'; user: AdminUser } | null>(null);
	let confirmOpen = $state(false);

	function ask(kind: 'disable' | 'delete', user: AdminUser) {
		confirming = { kind, user };
		confirmOpen = true;
	}

	function succeeded(action: AdminAction): string {
		const username = action.user.username;
		if (action.kind === 'delete') return m.admin_deleted({ username });
		if (action.change.disabled === true) return m.admin_disabled_toast({ username });
		if (action.change.disabled === false) return m.admin_enabled_toast({ username });
		return action.change.role === 'admin'
			? m.admin_promoted_toast({ username })
			: m.admin_demoted_toast({ username });
	}

	function run(action: AdminAction) {
		mutation.mutate(action, {
			onSuccess: () => {
				toast.success(succeeded(action));
				// A page emptied by a delete moves back one page.
				if (action.kind === 'delete' && rows.length === 1 && pageIndex > 0) pageIndex -= 1;
			},
			onError: (error) =>
				toast.error(
					error instanceof ApiError && error.code === 'last_admin'
						? m.admin_error_last_admin()
						: m.video_action_failed()
				)
		});
	}

	function confirm() {
		if (!confirming) return;
		const { kind, user } = confirming;
		confirmOpen = false;
		run(kind === 'delete' ? { kind, user } : { kind: 'update', user, change: { disabled: true } });
	}
</script>

<svelte:head><title>{pageTitle(m.admin_users())}</title></svelte:head>

<div class="flex flex-wrap items-center justify-between gap-3">
	<div class="relative w-full sm:max-w-xs">
		<SearchIcon
			class="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
			aria-hidden="true"
		/>
		<Input
			type="search"
			bind:value={input}
			placeholder={m.admin_search()}
			aria-label={m.admin_search()}
			class="ps-9"
			autocomplete="off"
			data-testid="admin-search"
		/>
	</div>
	{#if users.data}
		<p class="text-sm text-muted-foreground tabular" data-testid="admin-user-count">
			{m.admin_user_count({ count: total })}
		</p>
	{/if}
</div>

{#if users.isPending}
	<div class="flex flex-col gap-2" role="status" aria-label={m.admin_loading()}>
		{#each { length: 6 }, i (i)}<Skeleton class="h-11 w-full" />{/each}
	</div>
{:else if users.isError && !users.data}
	<ErrorState onretry={() => void users.refetch()} />
{:else if rows.length === 0}
	<EmptyState icon={query ? SearchIcon : UsersIcon} title={m.admin_no_users()} />
{:else}
	<div class="rounded-xl border">
		<Table.Root data-testid="admin-users">
			<Table.Header>
				<Table.Row class="hover:bg-transparent">
					<Table.Head class="ps-4">{m.admin_col_user()}</Table.Head>
					<Table.Head>{m.admin_col_role()}</Table.Head>
					<Table.Head class="hidden md:table-cell">{m.admin_col_sign_in()}</Table.Head>
					<Table.Head class="hidden lg:table-cell">{m.admin_col_created()}</Table.Head>
					<Table.Head class="hidden sm:table-cell">{m.admin_col_last_seen()}</Table.Head>
					<Table.Head class="hidden text-end lg:table-cell">
						{m.admin_col_subscriptions()}
					</Table.Head>
					<Table.Head class="w-12 pe-2">
						<span class="sr-only">{m.admin_col_actions()}</span>
					</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each rows as user (user.id)}
					{@const self = user.id === session.account?.id}
					<Table.Row class="h-12" data-testid="admin-user-row" data-username={user.username}>
						<Table.Cell class="max-w-48 ps-4">
							<div class="flex min-w-0 items-center gap-2">
								<span class="truncate font-medium">{user.username}</span>
								{#if self}<Badge variant="secondary">{m.admin_you()}</Badge>{/if}
								{#if user.disabled}
									<Badge variant="destructive" data-testid="admin-disabled">
										{m.admin_disabled()}
									</Badge>
								{/if}
							</div>
						</Table.Cell>
						<Table.Cell data-testid="admin-role">
							{#if user.role === 'admin'}
								<Badge variant="outline"><ShieldIcon />{m.admin_role_admin()}</Badge>
							{:else}
								<span class="text-muted-foreground">{m.admin_role_user()}</span>
							{/if}
						</Table.Cell>
						<Table.Cell class="hidden text-muted-foreground md:table-cell">
							{user.auth_method === 'oidc' ? m.admin_auth_oidc() : m.admin_auth_local()}
						</Table.Cell>
						<Table.Cell class="hidden text-muted-foreground lg:table-cell">
							{formatDate(user.created_at, locale)}
						</Table.Cell>
						<Table.Cell class="hidden text-muted-foreground sm:table-cell">
							{#if user.last_seen_at}
								<time datetime={user.last_seen_at} title={formatDate(user.last_seen_at, locale)}>
									{formatRelativeTime(user.last_seen_at, locale)}
								</time>
							{:else}
								{m.admin_never()}
							{/if}
						</Table.Cell>
						<Table.Cell class="hidden text-end text-muted-foreground tabular lg:table-cell">
							{formatNumber(user.subscriptions, locale)}
						</Table.Cell>
						<Table.Cell class="pe-2 text-end">
							{#if !self}
								<DropdownMenu.Root>
									<DropdownMenu.Trigger
										class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })}
										aria-label={m.admin_user_actions({ username: user.username })}
										data-testid="admin-user-menu"
									>
										<EllipsisIcon />
									</DropdownMenu.Trigger>
									<DropdownMenu.Content
										align="end"
										class="w-max min-w-44 [&_[data-slot=dropdown-menu-item]]:whitespace-nowrap"
									>
										{#if user.role === 'admin'}
											<DropdownMenu.Item
												onSelect={() => run({ kind: 'update', user, change: { role: 'user' } })}
											>
												<ShieldOffIcon />{m.admin_make_user()}
											</DropdownMenu.Item>
										{:else}
											<DropdownMenu.Item
												onSelect={() => run({ kind: 'update', user, change: { role: 'admin' } })}
											>
												<ShieldIcon />{m.admin_make_admin()}
											</DropdownMenu.Item>
										{/if}
										{#if user.disabled}
											<DropdownMenu.Item
												onSelect={() => run({ kind: 'update', user, change: { disabled: false } })}
											>
												<UserCheckIcon />{m.admin_enable()}
											</DropdownMenu.Item>
										{:else}
											<DropdownMenu.Item onSelect={() => ask('disable', user)}>
												<UserXIcon />{m.admin_disable()}
											</DropdownMenu.Item>
										{/if}
										<DropdownMenu.Separator />
										<DropdownMenu.Item variant="destructive" onSelect={() => ask('delete', user)}>
											<Trash2Icon />{m.admin_delete()}
										</DropdownMenu.Item>
									</DropdownMenu.Content>
								</DropdownMenu.Root>
							{/if}
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	</div>

	{#if total > ADMIN_PAGE_SIZE}
		<div class="flex items-center justify-end gap-2">
			<span class="text-sm text-muted-foreground tabular">
				{m.admin_range({ from, to, total })}
			</span>
			<Button
				variant="outline"
				size="icon-sm"
				aria-label={m.admin_previous()}
				disabled={pageIndex === 0}
				onclick={() => (pageIndex -= 1)}
			>
				<ChevronLeftIcon />
			</Button>
			<Button
				variant="outline"
				size="icon-sm"
				aria-label={m.admin_next()}
				disabled={to >= total}
				onclick={() => (pageIndex += 1)}
			>
				<ChevronRightIcon />
			</Button>
		</div>
	{/if}
{/if}

<AlertDialog.Root bind:open={confirmOpen}>
	<AlertDialog.Content>
		{#if confirming}
			{@const username = confirming.user.username}
			<AlertDialog.Header>
				<AlertDialog.Title>
					{confirming.kind === 'delete'
						? m.admin_delete_title({ username })
						: m.admin_disable_title({ username })}
				</AlertDialog.Title>
				<AlertDialog.Description>
					{confirming.kind === 'delete' ? m.admin_delete_body() : m.admin_disable_body()}
				</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
				<AlertDialog.Action variant="destructive" onclick={confirm} data-testid="admin-confirm">
					{confirming.kind === 'delete' ? m.admin_delete() : m.admin_disable()}
				</AlertDialog.Action>
			</AlertDialog.Footer>
		{/if}
	</AlertDialog.Content>
</AlertDialog.Root>
