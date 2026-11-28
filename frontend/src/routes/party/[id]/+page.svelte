<script lang="ts">
	import DoorClosedIcon from '@lucide/svelte/icons/door-closed';
	import UserRoundXIcon from '@lucide/svelte/icons/user-round-x';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { ApiError, isNotFound } from '#lib/api/mapping.js';
	import { createRoomQuery } from '#lib/api/rooms.js';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import JoinForm from '#lib/components/together/join-form.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { party } from '#lib/together/party.svelte.js';
	import { saveGuestName, savedGuestName } from '#lib/together/sync.js';

	const roomId = $derived(page.params.id ?? '');
	const debug = $derived(page.url.searchParams.get('debug') === 'sync');
	const info = createRoomQuery(() => roomId);

	let guestName = $state<string | null>(null);

	const ended = $derived(party.ended?.id === roomId ? party.ended.status : null);
	// Who joins: the account, or a guest once they picked a name.
	const joinAs = $derived.by(() => {
		if (!session.ready || !info.data || ended) return null;
		if (session.account) return { name: undefined };
		return guestName ? { name: guestName } : null;
	});

	$effect(() => {
		const as = joinAs;
		if (!as) return;
		untrack(() => party.join(roomId, { name: as.name, debug }));
	});

	// Joined: on to the watch page of what the party is playing.
	$effect(() => {
		if (party.id !== roomId || party.status !== 'open' || !party.room) return;
		const current = party.current;
		untrack(() => {
			party.panelOpen = true;
			if (current) void goto(resolve(`/watch?v=${current.video.id}`), { replaceState: true });
			else void goto(resolve('/'), { replaceState: true });
		});
	});

	function join(name: string) {
		saveGuestName(name);
		guestName = name;
	}

	const loginRequired = $derived(info.error instanceof ApiError && info.error.status === 401);
	const title = $derived(
		pageTitle(info.data ? m.party_owner_title({ owner: info.data.owner }) : m.party_title())
	);
</script>

<svelte:head>
	<title>{title}</title>
</svelte:head>

{#snippet notice(icon: typeof DoorClosedIcon, heading: string, body?: string, withSignIn = false)}
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
	{@render notice(UsersRoundIcon, m.party_title(), m.party_login_required(), true)}
{:else if (info.isError && isNotFound(info.error)) || ended === 'not_found'}
	{@render notice(DoorClosedIcon, m.party_not_found_title(), m.party_not_found_body())}
{:else if info.isError}
	<div class="px-(--page-gutter) py-6">
		<ErrorState
			title={m.party_error_load()}
			description={m.error_load_body()}
			onretry={() => void info.refetch()}
		/>
	</div>
{:else if ended === 'closed'}
	{@render notice(DoorClosedIcon, m.party_closed_title())}
{:else if ended === 'kicked'}
	{@render notice(UserRoundXIcon, m.party_kicked_title(), m.party_kicked_body())}
{:else if ended === 'full'}
	{@render notice(UsersRoundIcon, m.party_full_title(), m.party_full_body())}
{:else if ended === 'rejected'}
	{@render notice(UsersRoundIcon, m.party_title(), m.party_rejected())}
{:else if info.data && session.ready && !session.account && !guestName}
	<div class="px-(--page-gutter) py-10">
		<JoinForm owner={info.data.owner} name={savedGuestName()} onjoin={join} />
	</div>
{:else}
	<div class="flex flex-1 items-center justify-center px-(--page-gutter) py-16">
		<p
			class="flex items-center gap-2 text-sm text-muted-foreground"
			role="status"
			data-testid="party-joining"
		>
			<span class="size-1.5 animate-pulse rounded-full bg-primary" aria-hidden="true"></span>
			{m.party_joining()}
		</p>
	</div>
{/if}
