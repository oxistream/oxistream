<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { changePassword, deleteAccount } from '#lib/api/settings.js';
	import { session } from '#lib/session.svelte.js';
	import AccountSettings from '../account-settings.svelte';

	async function remove(body: { username: string; password?: string }) {
		const result = await deleteAccount(body);
		if (result.ok) {
			await session.signOut();
			await goto(resolve('/'));
		}
		return result;
	}
</script>

{#if session.account}
	<AccountSettings
		account={session.account}
		localEnabled={session.capabilities?.local_enabled !== false}
		onchangepassword={changePassword}
		ondelete={remove}
	/>
{/if}
