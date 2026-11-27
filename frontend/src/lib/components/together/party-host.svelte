<script lang="ts">
	import PlayIcon from '@lucide/svelte/icons/play';
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import { createSettingsQuery, playbackPreferences } from '#lib/api/settings.js';
	import { useVideo } from '#lib/api/video.js';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { party, type PartyEnd } from '#lib/together/party.svelte.js';
	import PartySheet from './party-sheet.svelte';
	import SyncReadout from './sync-readout.svelte';

	party.overlay = overlay;

	const currentId = $derived(party.current?.video.id ?? null);
	const video = useVideo(() => currentId);
	const settings = createSettingsQuery();
	const settingsKnown = $derived(session.ready && (!session.user || !settings.isPending));

	$effect(() => {
		if (session.ready) untrack(() => party.restore(session.account !== null));
	});

	$effect(() => {
		const current = party.current;
		const details = video.data?.id === current?.video.id ? video.data : undefined;
		const ready = settingsKnown && (details !== undefined || video.isError);
		const playback = settingsKnown ? playbackPreferences(settings.data) : undefined;
		void party.room;
		untrack(() => party.show(current, details, ready, playback));
	});

	$effect(() => {
		const room = party.room;
		const sync = party.sync;
		if (!room || !sync) return;
		const playback = {
			playing: room.playing,
			position: room.position,
			positionAt: room.position_at
		};
		untrack(() => sync.update(playback));
	});

	$effect(() => {
		const connection = party.connection;
		if (connection?.final) {
			const status = connection.status as PartyEnd['status'];
			untrack(() => party.finish(status));
		}
	});

	// The invite page explains it in place.
	const notice = $derived(
		party.ended && !(page.route.id === '/party/[id]' && page.params.id === party.ended.id)
			? party.ended.status
			: null
	);
	const notices: Record<PartyEnd['status'], [() => string, () => string]> = {
		closed: [m.party_closed_title, m.party_closed_body],
		kicked: [m.party_kicked_title, m.party_kicked_body],
		not_found: [m.party_not_found_title, m.party_not_found_body],
		full: [m.party_full_title, m.party_full_body],
		rejected: [m.party_title, m.party_rejected]
	};
</script>

{#snippet overlay()}
	{@const sync = party.sync}
	{#if sync}
		{#if party.debug}
			<SyncReadout stats={sync.stats} class="absolute start-2 top-2 z-10" />
		{/if}
		{#if sync.blocked}
			<div class="absolute inset-0 z-10 flex items-center justify-center rounded-xl bg-black/40">
				<Button size="lg" class="rounded-full px-5" onclick={() => sync.join()}>
					<PlayIcon />
					{m.party_sync_play()}
				</Button>
			</div>
		{/if}
	{/if}
{/snippet}

<PartySheet bind:open={party.sheetOpen} />

<AlertDialog.Root bind:open={party.leaveOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{m.party_leave_title()}</AlertDialog.Title>
			<AlertDialog.Description>{m.party_leave_body()}</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
			<AlertDialog.Action
				class={buttonVariants({ variant: 'destructive' })}
				onclick={() => party.leave()}
				data-testid="party-leave-confirm"
			>
				{m.party_leave()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<AlertDialog.Root
	open={notice !== null}
	onOpenChange={(open) => {
		if (!open) party.ended = null;
	}}
>
	<AlertDialog.Content data-testid="party-ended">
		{#if notice}
			<AlertDialog.Header>
				<AlertDialog.Title>{notices[notice][0]()}</AlertDialog.Title>
				<AlertDialog.Description>{notices[notice][1]()}</AlertDialog.Description>
			</AlertDialog.Header>
		{/if}
		<AlertDialog.Footer>
			<AlertDialog.Action onclick={() => (party.ended = null)}>{m.party_ok()}</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
