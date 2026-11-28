<script lang="ts">
	import FolderIcon from '@lucide/svelte/icons/folder';
	import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
	import type { SubscriptionGroup } from '#lib/api/subscription-group-list.js';
	import {
		createCreateGroupMutation,
		createGroupMembershipMutation
	} from '#lib/api/subscription-groups.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import * as Popover from '#lib/components/ui/popover/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import GroupNameForm from './group-name-form.svelte';

	let {
		channelId,
		channelName,
		groups
	}: {
		channelId: string;
		channelName: string;
		groups: SubscriptionGroup[];
	} = $props();

	const uid = $props.id();
	const membership = createGroupMembershipMutation();
	const create = createCreateGroupMutation();
	let open = $state(false);
	let failed = $state(false);

	const memberOf = $derived(groups.filter((g) => g.channelIds.includes(channelId)));

	function toggle(groupId: string, member: boolean) {
		failed = false;
		membership.mutate({ groupId, channelId, member }, { onError: () => (failed = true) });
	}
</script>

<Popover.Root bind:open onOpenChange={(next) => next && (failed = false)}>
	<Popover.Trigger
		class={cn(
			'inline-flex h-8 shrink-0 items-center gap-1 rounded-md px-2 text-sm font-medium text-muted-foreground tabular transition-colors duration-(--motion-in) ease-ui outline-none hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring',
			memberOf.length > 0 && 'text-foreground'
		)}
		aria-label={m.groups_for_channel({ name: channelName })}
		data-testid="group-picker"
	>
		{#if memberOf.length > 0}
			<FolderIcon class="size-4" aria-hidden="true" />
			<span>{memberOf.length}</span>
		{:else}
			<FolderPlusIcon class="size-4" aria-hidden="true" />
		{/if}
	</Popover.Trigger>
	<Popover.Content align="end" class="flex w-[min(17rem,calc(100vw-2rem))] flex-col gap-2 p-2">
		{#if groups.length}
			<ul
				class="flex max-h-64 flex-col overflow-y-auto overscroll-contain"
				data-testid="group-picker-list"
			>
				{#each groups as group (group.id)}
					{@const id = `${uid}-${group.id}`}
					<li
						class="flex min-h-9 items-center gap-2.5 rounded-sm px-2 transition-colors duration-(--motion-in) ease-ui hover:bg-accent"
					>
						<Checkbox
							{id}
							checked={group.channelIds.includes(channelId)}
							onCheckedChange={(checked) => toggle(group.id, checked)}
						/>
						<Label for={id} class="min-w-0 flex-1 cursor-pointer py-1.5 font-normal">
							<span class="truncate">{group.name}</span>
						</Label>
					</li>
				{/each}
			</ul>
		{/if}
		{#if failed}
			<p class="px-2 text-xs font-medium text-destructive" role="alert">
				{m.video_action_failed()}
			</p>
		{/if}
		<GroupNameForm
			{groups}
			label={m.groups_new()}
			submitLabel={m.groups_create()}
			onsubmit={(name) => create.mutateAsync({ name, channelIds: [channelId] })}
		/>
	</Popover.Content>
</Popover.Root>
