<script lang="ts">
	import DoorClosedIcon from '@lucide/svelte/icons/door-closed';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import ShuffleIcon from '@lucide/svelte/icons/shuffle';
	import XIcon from '@lucide/svelte/icons/x';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		createCloseRoomMutation,
		createCreateRoomMutation,
		createRoomsQuery
	} from '#lib/api/rooms.js';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import * as Avatar from '#lib/components/ui/avatar/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Sheet from '#lib/components/ui/sheet/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';
	import { cn } from '#lib/utils.js';
	import { initials } from '#lib/video.js';
	import { addErrorMessage } from './errors.js';
	import PartyAdd from './party-add.svelte';
	import PartyLibrary from './party-library.svelte';
	import PartyMembers from './party-members.svelte';
	import PartyNowPlaying from './party-now-playing.svelte';
	import PartyQueue from './party-queue.svelte';
	import PartyStatus from './party-status.svelte';
	import ShareButton from './share-button.svelte';

	const ERROR_VISIBLE_MS = 6000;
	const AVATARS_SHOWN = 4;

	let {
		open,
		dialog = false,
		onclose
	}: {
		open: boolean;
		/** Inside the sheet, which brings its own title and close button. */
		dialog?: boolean;
		onclose: () => void;
	} = $props();

	const uid = $props.id();
	const locale = getLocale();
	const rooms = createRoomsQuery();
	const create = createCreateRoomMutation();
	const closeRoom = createCloseRoomMutation();
	let endOpen = $state(false);
	let tab = $state<'queue' | 'members'>('queue');
	let now = $state(Date.now());

	$effect(() => {
		if (open && !party.active && session.user) void rooms.refetch();
	});

	const queue = $derived(party.room?.queue ?? []);
	const participants = $derived(party.participants);
	const shown = $derived(
		participants.length > AVATARS_SHOWN ? participants.slice(0, AVATARS_SHOWN - 1) : participants
	);

	const lastError = $derived(party.connection?.lastError);
	const errorMessage = $derived(
		lastError && now - lastError.at < ERROR_VISIBLE_MS ? addErrorMessage(lastError.code) : null
	);
	const lastAdded = $derived(party.connection?.lastAdded);
	const addedMessage = $derived(
		lastAdded && now - lastAdded.at < ERROR_VISIBLE_MS
			? lastAdded.skipped > 0
				? m.party_batch_skipped({
						added: formatNumber(lastAdded.added, locale),
						skipped: formatNumber(lastAdded.skipped, locale)
					})
				: m.party_batch_added({ added: formatNumber(lastAdded.added, locale) })
			: null
	);
	/** Videos playing or queued, marked where they can be added. */
	const queued = $derived(
		new Set([...(party.current ? [party.current] : []), ...queue].map((e) => e.video.id))
	);
	$effect(() => {
		const at = Math.max(lastError?.at ?? 0, lastAdded?.at ?? 0);
		if (!at) return;
		now = Date.now();
		const timer = setTimeout(() => (now = Date.now()), ERROR_VISIBLE_MS);
		return () => clearTimeout(timer);
	});

	async function start() {
		// The video playing now becomes the party's first.
		const videoId = appPlayer.claim ? undefined : appPlayer.media?.videoId;
		try {
			const room = await create.mutateAsync(videoId);
			party.start(room.id, videoId);
		} catch {
			// Shown below the button.
		}
	}

	async function end() {
		const id = party.id;
		if (!id) return;
		try {
			await closeRoom.mutateAsync(id);
			endOpen = false;
			party.leave();
		} catch {
			// Shown in the dialog.
		}
	}
</script>

{#snippet count(value: number)}
	<span class="text-xs text-muted-foreground tabular-nums">{formatNumber(value, locale)}</span>
{/snippet}

<div class="flex min-h-0 flex-1 flex-col" data-testid="party-panel">
	<!-- The sheet's own close button sits in the top corner, so the row ends before it. -->
	<header class={cn('flex h-16 shrink-0 items-center gap-1 p-2 ps-4', dialog && 'pe-13')}>
		{#if dialog}
			<Sheet.Title class="text-base font-semibold">{m.party_heading()}</Sheet.Title>
		{:else}
			<h2 class="text-base font-semibold text-foreground">{m.party_heading()}</h2>
		{/if}
		{#if party.active}
			<PartyStatus status={party.status} class="ms-1" />
		{/if}
		<div class="ms-auto flex items-center gap-1">
			{#if party.active}
				<button
					type="button"
					class="me-1 flex rounded-full outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring"
					aria-label="{m.party_show_members()} ({formatNumber(participants.length, locale)})"
					title={m.party_show_members()}
					onclick={() => (tab = 'members')}
					data-testid="party-avatars"
				>
					<Avatar.Group class="*:data-[slot=avatar]:ring-sidebar">
						{#each shown as person (person.id)}
							<Avatar.Root size="sm" class="bg-sidebar-accent">
								<Avatar.Fallback
									class="bg-sidebar-accent text-[0.625rem] font-medium text-sidebar-accent-foreground"
								>
									{initials(person.name)}
								</Avatar.Fallback>
							</Avatar.Root>
						{/each}
						{#if participants.length > shown.length}
							<Avatar.GroupCount class="size-6 text-[0.625rem] ring-sidebar">
								+{formatNumber(participants.length - shown.length, locale)}
							</Avatar.GroupCount>
						{/if}
					</Avatar.Group>
				</button>
				<ShareButton url={party.inviteUrl} />
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon-sm"
								aria-label={m.party_menu()}
								title={m.party_menu()}
								data-testid="party-menu"
							>
								<EllipsisVerticalIcon />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end" class="w-auto min-w-44">
						<DropdownMenu.Item onSelect={() => party.leave()} data-testid="party-leave">
							<LogOutIcon />
							{m.party_leave()}
						</DropdownMenu.Item>
						{#if party.isOwner}
							<DropdownMenu.Separator />
							<DropdownMenu.Item
								variant="destructive"
								onSelect={() => (endOpen = true)}
								data-testid="party-end"
							>
								<DoorClosedIcon />
								{m.party_end()}
							</DropdownMenu.Item>
						{/if}
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			{/if}
			{#if !dialog}
				<Button variant="ghost" size="icon-sm" onclick={onclose}>
					<XIcon />
					<span class="sr-only">{m.close()}</span>
				</Button>
			{/if}
		</div>
	</header>

	<div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-2 pb-2">
		{#if party.active}
			<PartyNowPlaying current={party.current} class="px-2" />

			{#if errorMessage}
				<p class="px-2 text-sm font-medium text-destructive" role="alert">{errorMessage}</p>
			{/if}
			<p
				class="px-2 text-sm text-muted-foreground empty:hidden"
				role="status"
				data-testid="party-batch"
			>
				{addedMessage ?? ''}
			</p>

			<Tabs.Root bind:value={tab} class="gap-1">
				<Tabs.List
					variant="line"
					class="sticky top-0 z-10 w-full shrink-0 justify-start bg-sidebar px-2"
				>
					<Tabs.Trigger value="queue" class="flex-none px-1.5" data-testid="party-tab-queue">
						{m.party_queue()}
						{@render count(queue.length)}
					</Tabs.Trigger>
					<Tabs.Trigger value="members" class="flex-none px-1.5" data-testid="party-tab-members">
						{m.party_members()}
						{@render count(participants.length)}
					</Tabs.Trigger>
				</Tabs.List>
				<Tabs.Content value="queue" class="flex flex-col gap-1">
					<div class="flex flex-col gap-1 px-1 pt-1">
						<PartyAdd {queued} onadd={(id) => party.add(id)} />
						<div class="flex items-center gap-1">
							{#if session.user}
								<PartyLibrary
									{queued}
									onadd={(id) => party.add(id)}
									onaddmany={(ids) => party.addMany(ids)}
								/>
							{/if}
							<Button
								variant="ghost"
								size="sm"
								class="ms-auto whitespace-nowrap text-muted-foreground hover:text-foreground"
								disabled={queue.length < 2}
								onclick={() => party.shuffle()}
								data-testid="party-shuffle"
							>
								<ShuffleIcon />
								{m.party_shuffle()}
							</Button>
						</div>
					</div>
					<PartyQueue
						{queue}
						revision={party.room?.revision}
						onselect={(entry) => party.send({ type: 'select', entry: entry.id })}
						onremove={(entry) => party.send({ type: 'remove', entry: entry.id })}
						onmove={(entry, to) => party.move(entry.id, to)}
					/>
				</Tabs.Content>
				<Tabs.Content value="members">
					<PartyMembers
						{participants}
						meId={party.connection?.participantId}
						canKick={party.isOwner}
						onkick={(person) => party.send({ type: 'kick', participant: person.id })}
					/>
				</Tabs.Content>
			</Tabs.Root>
		{:else if !session.ready}
			<Skeleton class="mx-2 h-9" />
		{:else if !session.user}
			<div class="flex flex-col gap-3 px-2">
				<p class="text-sm text-pretty text-muted-foreground">{m.party_sign_in()}</p>
				<SignInActions />
			</div>
		{:else}
			<div class="flex flex-col gap-2 px-2">
				<Button onclick={start} disabled={create.isPending} data-testid="party-start">
					<PlusIcon />
					{m.party_start()}
				</Button>
				{#if create.isError}
					<p class="text-sm font-medium text-destructive" role="alert">
						{m.party_start_failed()}
					</p>
				{/if}
			</div>

			{#if rooms.isError}
				<p class="px-2 text-sm text-muted-foreground">{m.party_yours_error()}</p>
			{:else if rooms.data?.length}
				<section class="flex flex-col" aria-labelledby="{uid}-yours">
					<h3
						id="{uid}-yours"
						class="flex h-8 items-center px-2 text-xs font-medium text-sidebar-foreground/70"
					>
						{m.party_yours()}
					</h3>
					<ul class="flex flex-col gap-0.5">
						{#each rooms.data as room (room.id)}
							<li
								class="flex items-center gap-3 rounded-md p-2 transition-colors hover:bg-sidebar-accent"
							>
								<div class="min-w-0 flex-1">
									<p class="truncate text-sm font-medium">
										{room.currentTitle ?? m.party_nothing()}
									</p>
									<p class="truncate text-xs text-muted-foreground">
										{m.party_meta({
											people: formatNumber(room.participantCount, locale),
											queue: formatNumber(room.queueCount, locale)
										})}
									</p>
								</div>
								<Button
									variant="outline"
									size="sm"
									onclick={() => goto(resolve('/party/[id]', { id: room.id }))}
								>
									{m.party_rejoin()}
								</Button>
							</li>
						{/each}
					</ul>
				</section>
			{/if}
		{/if}
	</div>
</div>

<AlertDialog.Root bind:open={endOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{m.party_end_title()}</AlertDialog.Title>
			<AlertDialog.Description>{m.party_end_body()}</AlertDialog.Description>
		</AlertDialog.Header>
		{#if closeRoom.isError}
			<p class="text-sm font-medium text-destructive" role="alert">{m.party_end_failed()}</p>
		{/if}
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
			<AlertDialog.Action
				class={buttonVariants({ variant: 'destructive' })}
				disabled={closeRoom.isPending}
				onclick={end}
			>
				{m.party_end()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
