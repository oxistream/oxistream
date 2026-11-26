<script lang="ts">
	import { page } from '$app/state';
	import {
		createSubscriptionMutation,
		createSubscriptionStateQuery
	} from '#lib/api/subscriptions.js';
	import { authHref } from '#lib/auth/links.js';
	import { session } from '#lib/session.svelte.js';
	import SubscribeToggle from './subscribe-toggle.svelte';

	let {
		channelId,
		size = 'default',
		class: className
	}: { channelId: string; size?: 'default' | 'sm'; class?: string } = $props();

	const status = createSubscriptionStateQuery(() => channelId);
	const mutation = createSubscriptionMutation();

	const signInHref = $derived(
		session.user ? undefined : authHref('/login', page.url.pathname + page.url.search)
	);
	const subscribed = $derived(status.data === true);
</script>

<SubscribeToggle
	{subscribed}
	{signInHref}
	{size}
	class={className}
	loading={session.user !== null && status.isPending}
	onclick={() => mutation.mutate({ channelId, subscribe: !subscribed })}
/>
