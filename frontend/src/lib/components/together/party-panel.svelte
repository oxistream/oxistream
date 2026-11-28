<script lang="ts">
	import DoorClosedIcon from '@lucide/svelte/icons/door-closed';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import ShuffleIcon from '@lucide/svelte/icons/shuffle';
	import XIcon from '@lucide/svelte/icons/x';
	import { untrack } from 'svelte';
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
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';
	import type { RoomPermission } from '#lib/together/connection.svelte.js';
	import { party } from '#lib/together/party.svelte.js';
	import { cn } from '#lib/utils.js';
	import { initials } from '#lib/video.js';
	import { addErrorMessage } from './errors.js';
	import PartyAdd from './party-add.svelte';
	import PartyChat from './party-chat.svelte';
	import PartyHistory from './party-history.svelte';
	import PartyLibrary from './party-library.svelte';
	import PartyMembers from './party-members.svelte';
	import PartyNowPlaying from './party-now-playing.svelte';
	import PartyQueue from './party-queue.svelte';
	import PartyStatus from './party-status.svelte';
	import PartyUpNext from './party-up-next.svelte';
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
	let now = $state(Date.now());

	$effect(() => {
		if (open && !party.active && session.user) void rooms.refetch();
	});

	$effect(() => {
		if (!open || party.tab !== 'chat') return;
		void party.chat.length;
		untrack(() => party.markChatRead());
	});

	const unread = $derived(party.unread);
	const permissionOptions: { value: RoomPermission; label: () => string }[] = [
		{ value: 'everyone', label: m.party_permission_everyone },
		{ value: 'host', label: m.party_permission_host }
	];
	const permissionGroups = [
		{ key: 'control', label: m.party_permission_control },
		{ key: 'add', label: m.party_permission_add }
	] as const;

	const queue = $derived(party.room?.queue ?? []);
	const upNext = $derived(party.room?.up_next ?? null);
	const playback = $derived(
		party.room
			? {
					playing: party.room.playing,
					position: party.room.position,
					positionAt: party.room.position_at
				}
			: undefined
	);
	const serverNow = () => party.connection?.serverNow() ?? Date.now();
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
	const presenceMessage = $derived.by(() => {
		const notice = party.presence;
		if (!notice) return '';
		const [first, second] = notice.names;
		const joined = notice.kind === 'joined';
		if (notice.names.length === 1) {
			return joined ? m.party_joined_one({ name: first }) : m.party_left_one({ name: first });
		}
		if (notice.names.length === 2) {
			return joined ? m.party_joined_two({ first, second }) : m.party_left_two({ first, second });
		}
		const name = notice.names.at(-1)!;
		const count = formatNumber(notice.names.length - 1, locale);
		return joined ? m.party_joined_many({ name, count }) : m.party_left_many({ name, count });
	});
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

<div class="relative flex min-h-0 flex-1 flex-col" data-testid="party-panel">
	<!-- Floats over the bottom, so nothing moves while it comes and goes. -->
	<p
		class={cn(
			'pointer-events-none absolute inset-x-3 bottom-3 z-20 truncate rounded-lg border bg-popover px-3 py-2 text-sm text-popover-foreground shadow-md empty:hidden',
			party.tab === 'chat' && 'bottom-24'
		)}
		role="status"
		data-testid="party-presence"
	>
		{party.active ? presenceMessage : ''}
	</p>
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
					onclick={() => (party.tab = 'members')}
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
						<DropdownMenu.Item
							onSelect={() => (party.locksOnLeave ? (party.leaveOpen = true) : party.leave())}
							data-testid="party-leave"
						>
							<LogOutIcon />
							{m.party_leave()}
						</DropdownMenu.Item>
						{#if party.isOwner}
							{#each permissionGroups as group (group.key)}
								<DropdownMenu.Separator />
								<DropdownMenu.Label>{group.label()}</DropdownMenu.Label>
								<DropdownMenu.RadioGroup
									value={party.permissions[group.key]}
									onValueChange={(value) =>
										party.setPermissions({ [group.key]: value as RoomPermission })}
								>
									{#each permissionOptions as option (option.value)}
										<DropdownMenu.RadioItem
											value={option.value}
											data-testid="party-permission-{group.key}-{option.value}"
										>
											{option.label()}
										</DropdownMenu.RadioItem>
									{/each}
								</DropdownMenu.RadioGroup>
							{/each}
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
			<PartyNowPlaying current={party.current} {playback} {serverNow} class="px-2" />

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

			<Tabs.Root bind:value={party.tab} class="gap-1">
				<Tabs.List
					variant="line"
					class="sticky top-0 z-10 w-full shrink-0 [scrollbar-width:none] justify-start gap-0 overflow-x-auto bg-sidebar px-0.5"
				>
					<Tabs.Trigger value="queue" class="flex-none gap-1 px-1" data-testid="party-tab-queue">
						{m.party_queue()}
						{@render count(queue.length)}
					</Tabs.Trigger>
					<Tabs.Trigger value="chat" class="flex-none gap-1 px-1" data-testid="party-tab-chat">
						{m.party_chat()}
						{#if unread > 0}
							<span
								class="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[0.625rem] leading-none font-semibold text-primary-foreground tabular-nums"
								data-testid="party-chat-unread"
							>
								<span aria-hidden="true">
									{unread > 99 ? '99+' : formatNumber(unread, locale)}
								</span>
								<span class="sr-only">
									{m.party_chat_unread({ count: formatNumber(unread, locale) })}
								</span>
							</span>
						{/if}
					</Tabs.Trigger>
					<Tabs.Trigger
						value="history"
						class="flex-none gap-1 px-1"
						data-testid="party-tab-history"
					>
						{m.party_history()}
					</Tabs.Trigger>
					<Tabs.Trigger
						value="members"
						class="flex-none gap-1 px-1"
						data-testid="party-tab-members"
					>
						{m.party_members()}
						{@render count(participants.length)}
					</Tabs.Trigger>
				</Tabs.List>
				<Tabs.Content value="queue" class="flex flex-col gap-1">
					{@const library = session.user !== null && party.canAdd}
					{#if party.canAdd || party.canControl}
						<div class="flex flex-col gap-1 px-1 pt-1">
							{#if party.canAdd}
								<PartyAdd {queued} onadd={(id) => party.add(id)} />
							{/if}
							{#if library || party.canControl}
								<div class="flex items-center gap-1">
									{#if library}
										<PartyLibrary
											{queued}
											onadd={(id) => party.add(id)}
											onaddmany={(ids) => party.addMany(ids)}
										/>
									{/if}
									{#if party.canControl}
										<Button
											variant="ghost"
											size="icon-sm"
											class="ms-auto text-muted-foreground hover:text-foreground"
											aria-label={m.party_shuffle()}
											title={m.party_shuffle()}
											disabled={queue.length < 2}
											onclick={() => party.shuffle()}
											data-testid="party-shuffle"
										>
											<ShuffleIcon />
										</Button>
										<label
											class="flex h-8 shrink-0 cursor-pointer items-center gap-2 rounded-md px-2 text-sm font-medium whitespace-nowrap text-muted-foreground hover:text-foreground"
										>
											{m.party_autoplay()}
											<Switch
												size="sm"
												checked={party.room?.autoplay ?? false}
												onCheckedChange={(enabled) => party.setAutoplay(enabled)}
												data-testid="party-autoplay"
											/>
										</label>
									{/if}
								</div>
							{/if}
						</div>
					{/if}
					{#if queue.length > 0 || !upNext}
						<PartyQueue
							{queue}
							revision={party.room?.revision}
							onselect={party.canControl
								? (entry) => party.send({ type: 'select', entry: entry.id })
								: undefined}
							onremove={(entry) => party.send({ type: 'remove', entry: entry.id })}
							canRemove={(entry) => party.canControl || entry.own}
							onmove={party.canControl ? (entry, to) => party.move(entry.id, to) : undefined}
						/>
					{/if}
					{#if upNext}
						<PartyUpNext
							video={upNext}
							onplay={party.canPlayNow(upNext.id) ? () => party.playNow(upNext.id) : undefined}
						/>
					{/if}
				</Tabs.Content>
				<Tabs.Content value="chat">
					<PartyChat
						messages={party.chat}
						disabled={party.status !== 'open'}
						onsend={(text) => party.sendChat(text)}
						onreact={(reaction) => party.react(reaction)}
					/>
				</Tabs.Content>
				<Tabs.Content value="history">
					<PartyHistory
						history={party.history}
						{queued}
						canPlay={(entry) => party.canPlayNow(entry.video.id)}
						canQueue={party.canAdd}
						onplay={(entry) => party.playNow(entry.video.id)}
						onqueue={(entry) => party.add(entry.video.id)}
					/>
				</Tabs.Content>
				<Tabs.Content value="members">
					<PartyMembers
						{participants}
						meId={party.connection?.participantId}
						canKick={party.isOwner}
						onkick={(person) => party.send({ type: 'kick', participant: person.id })}
						onresync={party.sync ? () => party.resync() : undefined}
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
