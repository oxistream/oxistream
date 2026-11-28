<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { page } from '$app/state';
	import { authHref } from '#lib/auth/links.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { MAX_NAME_LENGTH, cleanName } from '#lib/together/sync.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		owner,
		name = '',
		onjoin
	}: {
		owner: string;
		/** Prefilled from the last party this browser joined. */
		name?: string;
		onjoin: (name: string) => void;
	} = $props();

	const uid = $props.id();
	// svelte-ignore state_referenced_locally
	let value = $state(name);
	const cleaned = $derived(cleanName(value));
	const signInHref = $derived(authHref('/login', page.url.pathname));

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (cleaned) onjoin(cleaned);
	}
</script>

<div
	class="mx-auto flex w-full max-w-sm flex-col gap-5 rounded-xl border bg-card p-6"
	data-testid="join-form"
>
	<div class="flex flex-col items-center gap-3 text-center">
		<div
			class="flex size-11 items-center justify-center rounded-full bg-muted text-muted-foreground"
		>
			<UsersRoundIcon class="size-5" />
		</div>
		<h1 class="text-base font-semibold tracking-tight">{m.party_join_title({ owner })}</h1>
	</div>
	<form class="flex flex-col gap-3" onsubmit={submit}>
		<div class="flex flex-col gap-1.5">
			<Label for="{uid}-name">{m.party_join_name()}</Label>
			<Input
				id="{uid}-name"
				bind:value
				maxlength={MAX_NAME_LENGTH}
				autocomplete="nickname"
				required
			/>
		</div>
		<Button type="submit" disabled={!cleaned}>{m.party_join()}</Button>
	</form>
	<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved by authHref -->
	<a href={signInHref} class="text-center text-sm text-primary underline-offset-4 hover:underline">
		{m.party_join_sign_in()}
	</a>
</div>
