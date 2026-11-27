<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		createDeleteFeedTokenMutation,
		createFeedTokenMutation,
		createFeedTokenQuery
	} from '#lib/api/feed-token.js';
	import { changePassword, deleteAccount } from '#lib/api/settings.js';
	import { session } from '#lib/session.svelte.js';
	import AccountSettings from '../account-settings.svelte';
	import FeedTokenSettings from '../feed-token-settings.svelte';
	import SettingsError from './settings-error.svelte';

	const feedToken = createFeedTokenQuery();
	const createFeed = createFeedTokenMutation();
	const deleteFeed = createDeleteFeedTokenMutation();
	let feedUrl = $state<string | null>(null);

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
	>
		<FeedTokenSettings
			createdAt={feedToken.data?.created_at ?? null}
			url={feedUrl}
			disabled={feedToken.isPending || createFeed.isPending || deleteFeed.isPending}
			oncreate={() =>
				createFeed.mutate(undefined, { onSuccess: (created) => (feedUrl = created.url) })}
			ondelete={() => deleteFeed.mutate(undefined, { onSuccess: () => (feedUrl = null) })}
		/>
		<SettingsError failed={createFeed.isError || deleteFeed.isError} />
	</AccountSettings>
{/if}
