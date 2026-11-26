<script lang="ts">
	import DoorClosedIcon from '@lucide/svelte/icons/door-closed';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import UserRoundXIcon from '@lucide/svelte/icons/user-round-x';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { ApiError, isNotFound, toVideo } from '#lib/api/mapping.js';
	import { createCloseRoomMutation, createRoomQuery } from '#lib/api/rooms.js';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import JoinForm from '#lib/components/together/join-form.svelte';
	import RoomAdd from '#lib/components/together/room-add.svelte';
	import RoomParticipants from '#lib/components/together/room-participants.svelte';
	import RoomPlayer from '#lib/components/together/room-player.svelte';
	import RoomQueue from '#lib/components/together/room-queue.svelte';
	import RoomStatus from '#lib/components/together/room-status.svelte';
	import ShareButton from '#lib/components/together/share-button.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import {
		RoomConnection,
		type RoomErrorCode,
		type RoomParticipant
	} from '#lib/together/connection.svelte.js';
	import { guestKey, saveGuestName, savedGuestName } from '#lib/together/sync.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const ERROR_VISIBLE_MS = 6000;

	const roomId = $derived(page.params.room ?? '');
	const info = createRoomQuery(() => roomId);
	const closeRoom = createCloseRoomMutation();

	let guestName = $state<string | null>(null);
	let connection = $state<RoomConnection | null>(null);
	let closeOpen = $state(false);
	let now = $state(Date.now());

	// Who joins: the account, or a guest once they picked a name.
	const joinAs = $derived.by(() => {
		if (!session.ready || !info.data) return null;
		const account = session.account;
		if (account) return { key: `u:${account.id}`, name: undefined };
		return guestName ? { key: `g:${guestName}`, name: guestName } : null;
	});
	const joinKey = $derived(joinAs ? `${roomId}/${joinAs.key}` : null);

	$effect(() => {
		if (!joinKey) return;
		const name = joinAs?.name;
		const conn = new RoomConnection(roomId, name ? { name, guestKey: guestKey() } : {});
		conn.connect();
		connection = conn;
		return () => {
			conn.disconnect();
			connection = null;
		};
	});

	const room = $derived(connection?.room ?? null);
	const status = $derived(connection?.status ?? 'connecting');
	const current = $derived(room?.current ?? null);
	const currentVideo = $derived(current ? toVideo(current.video) : null);
	const isOwner = $derived(connection?.isOwner ?? info.data?.is_owner ?? false);
	const owner = $derived(info.data?.owner ?? room?.owner ?? '');
	const shareUrl = $derived(
		new URL(resolve('/together/[room]', { room: roomId }), page.url.origin).href
	);
	const title = $derived(
		pageTitle(
			(connection?.final ? null : currentVideo?.title) ??
				(owner ? m.together_room_title({ owner }) : m.together_title())
		)
	);

	const errorText: Partial<Record<RoomErrorCode, () => string>> = {
		queue_full: m.together_error_queue_full,
		video_unavailable: m.together_error_video,
		invalid: m.together_error_invalid,
		rate_limited: m.together_error_rate
	};
	const lastError = $derived(connection?.lastError);
	const errorMessage = $derived(
		lastError && now - lastError.at < ERROR_VISIBLE_MS
			? (errorText[lastError.code] ?? m.together_error_generic)()
			: null
	);
	$effect(() => {
		if (!lastError) return;
		now = Date.now();
		const timer = setTimeout(() => (now = Date.now()), ERROR_VISIBLE_MS);
		return () => clearTimeout(timer);
	});

	function join(name: string) {
		saveGuestName(name);
		guestName = name;
	}

	function kick(person: RoomParticipant) {
		connection?.send({ type: 'kick', participant: person.id });
	}

	async function confirmClose() {
		try {
			await closeRoom.mutateAsync(roomId);
			closeOpen = false;
			await goto(resolve('/together'));
		} catch {
			// Shown in the dialog.
		}
	}

	const loginRequired = $derived(info.error instanceof ApiError && info.error.status === 401);
	const final = $derived(connection?.final ? status : null);
</script>

<svelte:head>
	<title>{title}</title>
</svelte:head>

{#snippet notice(icon: typeof DoorClosedIcon, heading: string, body: string, withSignIn = false)}
	<div class="px-(--page-gutter) py-6">
		<EmptyState {icon} title={heading} description={body} headingLevel={2}>
			{#snippet actions()}
				{#if withSignIn}
					<SignInActions />
				{:else}
					<Button href={resolve('/')} variant="outline">{m.nav_home()}</Button>
				{/if}
			{/snippet}
		</EmptyState>
	</div>
{/snippet}

{#if loginRequired}
	{@render notice(UsersRoundIcon, m.together_title(), m.together_login_required(), true)}
{:else if (info.isError && isNotFound(info.error)) || final === 'not_found'}
	{@render notice(DoorClosedIcon, m.together_not_found_title(), m.together_not_found_body())}
{:else if info.isError}
	<div class="px-(--page-gutter) py-6">
		<ErrorState
			title={m.together_error_load()}
			description={m.error_load_body()}
			onretry={() => void info.refetch()}
		/>
	</div>
{:else if final === 'closed'}
	{@render notice(DoorClosedIcon, m.together_closed_title(), m.together_closed_body())}
{:else if final === 'kicked'}
	{@render notice(UserRoundXIcon, m.together_kicked_title(), m.together_kicked_body())}
{:else if final === 'full'}
	{@render notice(UsersRoundIcon, m.together_full_title(), m.together_full_body())}
{:else if final === 'rejected'}
	{@render notice(UsersRoundIcon, m.together_title(), m.together_error_generic())}
{:else if info.data && session.ready && !session.account && !guestName}
	<div class="px-(--page-gutter) py-10">
		<JoinForm owner={info.data.owner} name={savedGuestName()} onjoin={join} />
	</div>
{:else}
	<div class="@container flex-1">
		<div class="room">
			<div class="room-player">
				{#if !room}
					<Skeleton class="aspect-video w-full rounded-xl" />
				{:else if current && connection}
					{@const conn = connection}
					{#key current.id}
						<RoomPlayer
							videoId={current.video.id}
							playback={{
								playing: room.playing,
								position: room.position,
								positionAt: room.position_at
							}}
							serverNow={() => conn.serverNow()}
							onplay={(position) => conn.send({ type: 'play', position })}
							onpause={(position) => conn.send({ type: 'pause', position })}
							onseek={(position) => conn.send({ type: 'seek', position })}
							onended={() => conn.send({ type: 'ended', entry: current.id })}
						/>
					{/key}
				{:else}
					<div
						class="flex aspect-video w-full items-center justify-center rounded-xl border border-dashed"
					>
						<EmptyState
							icon={TvMinimalPlayIcon}
							title={m.together_no_video_title()}
							description={m.together_no_video_body()}
							class="border-0"
						/>
					</div>
				{/if}
			</div>

			<div class="room-meta flex flex-col gap-3">
				{#if currentVideo}
					<div class="flex flex-col gap-1">
						<h1 class="text-lg leading-snug font-semibold tracking-tight text-pretty">
							<a href={resolve(`/watch?v=${currentVideo.id}`)} class="hover:text-primary">
								{currentVideo.title}
							</a>
						</h1>
						{#if currentVideo.channel.id}
							<a
								href={resolve('/channel/[id]', { id: currentVideo.channel.id })}
								class="w-fit text-sm text-muted-foreground hover:text-foreground"
							>
								{currentVideo.channel.name}
							</a>
						{/if}
					</div>
				{/if}
				<div class="flex flex-wrap items-center gap-2">
					<span class="me-auto flex min-w-0 items-center gap-2">
						<span class="truncate text-sm font-medium">
							{owner ? m.together_room_title({ owner }) : m.together_title()}
						</span>
						<RoomStatus {status} />
					</span>
					<ShareButton url={shareUrl} />
					{#if isOwner}
						<Button variant="destructive" size="sm" onclick={() => (closeOpen = true)}>
							<DoorClosedIcon />
							{m.together_close()}
						</Button>
					{:else}
						<Button variant="ghost" size="sm" href={resolve(session.account ? '/together' : '/')}>
							<LogOutIcon />
							{m.together_leave()}
						</Button>
					{/if}
				</div>
				{#if session.account}
					<p class="text-xs text-muted-foreground">{m.together_history_hint()}</p>
				{/if}
			</div>

			<aside class="room-side flex min-h-0 flex-col gap-4">
				<RoomQueue
					{current}
					queue={room?.queue ?? []}
					onskip={(entry) => connection?.send({ type: 'skip', entry: entry.id })}
					onselect={(entry) => connection?.send({ type: 'select', entry: entry.id })}
					onremove={(entry) => connection?.send({ type: 'remove', entry: entry.id })}
					class="room-queue"
				>
					{#snippet add()}
						<RoomAdd
							error={errorMessage}
							onadd={(videoId) => connection?.send({ type: 'add', video_id: videoId })}
						/>
					{/snippet}
				</RoomQueue>
				<RoomParticipants
					participants={room?.participants ?? []}
					meId={connection?.participantId}
					canKick={isOwner}
					onkick={kick}
				/>
			</aside>
		</div>
	</div>

	<AlertDialog.Root bind:open={closeOpen}>
		<AlertDialog.Content>
			<AlertDialog.Header>
				<AlertDialog.Title>{m.together_close_title()}</AlertDialog.Title>
				<AlertDialog.Description>{m.together_close_body()}</AlertDialog.Description>
			</AlertDialog.Header>
			{#if closeRoom.isError}
				<p class="text-sm font-medium text-destructive" role="alert">
					{m.together_close_failed()}
				</p>
			{/if}
			<AlertDialog.Footer>
				<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
				<AlertDialog.Action
					class={buttonVariants({ variant: 'destructive' })}
					disabled={closeRoom.isPending}
					onclick={confirmClose}
				>
					{m.together_close()}
				</AlertDialog.Action>
			</AlertDialog.Footer>
		</AlertDialog.Content>
	</AlertDialog.Root>
{/if}

<style>
	.room {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-template-areas: 'player' 'meta' 'side';
		gap: 1rem 1.5rem;
		width: 100%;
		max-width: 112rem;
		margin-inline: auto;
		padding: 1rem var(--page-gutter) 2.5rem;
	}

	.room-player {
		grid-area: player;
	}

	.room-meta {
		grid-area: meta;
		min-width: 0;
	}

	.room-side {
		grid-area: side;
		min-width: 0;
	}

	@container (width >= 60rem) {
		.room {
			grid-template-columns: minmax(0, 1fr) 24rem;
			grid-template-rows: auto 1fr;
			grid-template-areas: 'player side' 'meta side';
		}

		.room :global(.room-queue) {
			max-height: min(40rem, calc(100svh - 14rem));
		}
	}
</style>
