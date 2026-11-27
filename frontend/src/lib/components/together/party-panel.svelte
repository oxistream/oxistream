<script lang="ts">
	import DoorClosedIcon from '@lucide/svelte/icons/door-closed';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import PlusIcon from '@lucide/svelte/icons/plus';
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
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Sheet from '#lib/components/ui/sheet/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';
	import { addErrorMessage } from './errors.js';
	import PartyMembers from './party-members.svelte';
	import PartyQueue from './party-queue.svelte';
	import PartyStatus from './party-status.svelte';
	import ShareButton from './share-button.svelte';

	const ERROR_VISIBLE_MS = 6000;

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

	const lastError = $derived(party.connection?.lastError);
	const errorMessage = $derived(
		lastError && now - lastError.at < ERROR_VISIBLE_MS ? addErrorMessage(lastError.code) : null
	);
	$effect(() => {
		if (!lastError) return;
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

{#snippet title(text: string, className?: string)}
	{#if dialog}
		<Sheet.Title class={className}>{text}</Sheet.Title>
	{:else}
		<h2 class={['font-medium text-foreground', className]}>{text}</h2>
	{/if}
{/snippet}

{#snippet description(text: string)}
	{#if dialog}
		<Sheet.Description>{text}</Sheet.Description>
	{:else}
		<p class="text-sm text-muted-foreground">{text}</p>
	{/if}
{/snippet}

<div class="flex min-h-0 flex-1 flex-col" data-testid="party-panel">
	<header class="relative flex flex-col gap-1.5 border-b border-sidebar-border p-4 pe-12">
		{#if party.active}
			<div class="flex min-w-0 items-center gap-2">
				{@render title(
					party.owner ? m.party_owner_title({ owner: party.owner }) : m.party_title(),
					'truncate'
				)}
				<PartyStatus status={party.status} class="shrink-0" />
			</div>
			{#if session.account}
				{@render description(m.party_history_hint())}
			{/if}
		{:else}
			{@render title(m.party_title())}
			{@render description(m.party_intro())}
		{/if}
		{#if !dialog}
			<Button variant="ghost" size="icon-sm" class="absolute end-3 top-3" onclick={onclose}>
				<XIcon />
				<span class="sr-only">{m.close()}</span>
			</Button>
		{/if}
	</header>

	<div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto p-4">
		{#if party.active}
			<div class="flex flex-col gap-1.5">
				<label for="{uid}-invite" class="px-1 text-sm font-semibold">{m.party_invite()}</label>
				<div class="flex gap-2">
					<Input
						id="{uid}-invite"
						value={party.inviteUrl}
						readonly
						class="min-w-0 flex-1 font-mono text-xs"
						onfocus={(event) => event.currentTarget.select()}
						data-testid="party-invite"
					/>
					<ShareButton url={party.inviteUrl} class="shrink-0" />
				</div>
			</div>

			{#if errorMessage}
				<p class="px-1 text-sm font-medium text-destructive" role="alert">{errorMessage}</p>
			{/if}

			<PartyQueue
				current={party.current}
				queue={party.room?.queue ?? []}
				onskip={(entry) => party.send({ type: 'skip', entry: entry.id })}
				onselect={(entry) => party.send({ type: 'select', entry: entry.id })}
				onremove={(entry) => party.send({ type: 'remove', entry: entry.id })}
			/>

			<PartyMembers
				participants={party.participants}
				meId={party.connection?.participantId}
				canKick={party.isOwner}
				onkick={(person) => party.send({ type: 'kick', participant: person.id })}
			/>
		{:else if !session.ready}
			<Skeleton class="h-9 w-full" />
		{:else if !session.user}
			<div class="flex flex-col gap-3">
				<p class="text-sm text-pretty text-muted-foreground">{m.party_sign_in()}</p>
				<SignInActions />
			</div>
		{:else}
			<div class="flex flex-col gap-2">
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
				<p class="text-sm text-muted-foreground">{m.party_yours_error()}</p>
			{:else if rooms.data?.length}
				<section class="flex flex-col gap-1.5" aria-labelledby="{uid}-yours">
					<h3 id="{uid}-yours" class="px-1 text-sm font-semibold">{m.party_yours()}</h3>
					<ul class="flex flex-col gap-0.5">
						{#each rooms.data as room (room.id)}
							<li class="flex items-center gap-3 rounded-lg p-2 hover:bg-muted/60">
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

	{#if party.active}
		<footer class="flex justify-end gap-2 border-t border-sidebar-border p-4">
			<Button variant="ghost" size="sm" onclick={() => party.leave()} data-testid="party-leave">
				<LogOutIcon />
				{m.party_leave()}
			</Button>
			{#if party.isOwner}
				<Button variant="destructive" size="sm" onclick={() => (endOpen = true)}>
					<DoorClosedIcon />
					{m.party_end()}
				</Button>
			{/if}
		</footer>
	{/if}
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
