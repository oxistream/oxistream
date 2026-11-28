<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import HouseIcon from '@lucide/svelte/icons/house';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { MAX_GROUPS, type SubscriptionGroup } from '#lib/api/subscription-group-list.js';
	import {
		createCreateGroupMutation,
		createDeleteGroupMutation,
		createMoveGroupMutation,
		createUpdateGroupMutation
	} from '#lib/api/subscription-groups.js';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import GroupNameForm from './group-name-form.svelte';

	let {
		open = $bindable(false),
		groups
	}: {
		open?: boolean;
		groups: SubscriptionGroup[];
	} = $props();

	const create = createCreateGroupMutation();
	const update = createUpdateGroupMutation();
	const remove = createDeleteGroupMutation();
	const move = createMoveGroupMutation();

	let renaming = $state<string>();
	let deleting = $state<SubscriptionGroup>();
	let failed = $state(false);

	$effect.pre(() => {
		if (!open) return;
		renaming = undefined;
		failed = false;
	});

	const onError = () => (failed = true);

	function setShowOnHome(group: SubscriptionGroup, showOnHome: boolean) {
		failed = false;
		update.mutate({ id: group.id, fields: { showOnHome } }, { onError });
	}

	function moveTo(group: SubscriptionGroup, index: number) {
		failed = false;
		move.mutate({ id: group.id, index }, { onError });
	}

	function confirmDelete() {
		if (!deleting) return;
		failed = false;
		remove.mutate(deleting.id, { onError });
		deleting = undefined;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="flex max-h-[min(40rem,calc(100dvh-2rem))] flex-col gap-4">
		<Dialog.Header>
			<Dialog.Title>{m.groups_label()}</Dialog.Title>
		</Dialog.Header>
		{#if groups.length === 0}
			<p class="text-sm text-muted-foreground">{m.groups_none()}</p>
		{:else}
			<ul
				class="-mx-2 flex min-h-0 flex-col overflow-y-auto overscroll-contain"
				aria-label={m.groups_label()}
				data-testid="group-manager-list"
			>
				<!-- By position, so moving a group does not move the row and its open menu. -->
				{#each groups as group, index (index)}
					<li
						class="flex min-h-12 items-center gap-2 rounded-md px-2 py-1"
						data-testid="group-manager-row"
					>
						{#if renaming === group.id}
							<GroupNameForm
								{groups}
								initial={group.name}
								exceptId={group.id}
								label={m.groups_name()}
								submitLabel={m.action_save()}
								autofocus
								class="flex-1"
								onsubmit={async (name) => {
									await update.mutateAsync({ id: group.id, fields: { name } });
									renaming = undefined;
								}}
								oncancel={() => (renaming = undefined)}
							/>
						{:else}
							<div class="flex min-w-0 flex-1 flex-col">
								<span class="truncate text-sm font-medium">{group.name}</span>
								<span class="text-xs text-muted-foreground tabular">
									{m.subscriptions_count({ count: group.channelIds.length })}
								</span>
							</div>
							<label class="flex shrink-0 cursor-pointer items-center gap-2 text-muted-foreground">
								<HouseIcon class="size-4" aria-hidden="true" />
								<span class="sr-only">{m.groups_show_on_home()}</span>
								<Switch
									checked={group.showOnHome}
									onCheckedChange={(checked) => setShowOnHome(group, checked)}
									aria-label={m.groups_show_on_home()}
									data-testid="group-show-on-home"
								/>
							</label>
							<DropdownMenu.Root>
								<DropdownMenu.Trigger
									class={cn(
										buttonVariants({ variant: 'ghost', size: 'icon-sm' }),
										'shrink-0 rounded-full text-muted-foreground hover:text-foreground data-[state=open]:bg-accent'
									)}
									aria-label={m.groups_actions({ name: group.name })}
									data-testid="group-actions"
								>
									<EllipsisVerticalIcon />
								</DropdownMenu.Trigger>
								<DropdownMenu.Content align="end" class="w-max min-w-44">
									<DropdownMenu.Item onSelect={() => (renaming = group.id)}>
										<PencilIcon />
										{m.groups_rename()}
									</DropdownMenu.Item>
									<DropdownMenu.Item
										disabled={index === 0}
										onSelect={() => moveTo(group, index - 1)}
									>
										<ArrowUpIcon />
										{m.groups_move_up()}
									</DropdownMenu.Item>
									<DropdownMenu.Item
										disabled={index === groups.length - 1}
										onSelect={() => moveTo(group, index + 1)}
									>
										<ArrowDownIcon />
										{m.groups_move_down()}
									</DropdownMenu.Item>
									<DropdownMenu.Separator />
									<DropdownMenu.Item variant="destructive" onSelect={() => (deleting = group)}>
										<Trash2Icon />
										{m.groups_delete()}
									</DropdownMenu.Item>
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
		{#if failed}
			<p class="text-sm font-medium text-destructive" role="alert">{m.video_action_failed()}</p>
		{/if}
		{#if groups.length < MAX_GROUPS}
			<GroupNameForm
				{groups}
				label={m.groups_new()}
				submitLabel={m.groups_create()}
				onsubmit={(name) => create.mutateAsync({ name })}
			/>
		{:else}
			<p class="text-sm text-muted-foreground">{m.groups_error_limit({ max: MAX_GROUPS })}</p>
		{/if}
	</Dialog.Content>
</Dialog.Root>

<AlertDialog.Root
	open={deleting !== undefined}
	onOpenChange={(next) => {
		if (!next) deleting = undefined;
	}}
>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{m.groups_delete_title({ name: deleting?.name ?? '' })}</AlertDialog.Title>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
			<AlertDialog.Action
				variant="destructive"
				onclick={confirmDelete}
				data-testid="group-delete-confirm"
			>
				{m.groups_delete()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
