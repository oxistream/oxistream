<script lang="ts">
	import {
		createSubscriptionMutation,
		createSubscriptionStateQuery
	} from '#lib/api/subscriptions.js';
	import { session } from '#lib/session.svelte.js';
	import SubscribeToggle from './subscribe-toggle.svelte';

	let {
		channelId,
		size = 'default',
		class: className
	}: { channelId: string; size?: 'default' | 'sm'; class?: string } = $props();

	const status = createSubscriptionStateQuery(() => channelId);
	const mutation = createSubscriptionMutation();

	const subscribed = $derived(status.data === true);
</script>

{#if session.user}
	<SubscribeToggle
		{subscribed}
		{size}
		class={className}
		loading={status.isPending}
		onclick={() => mutation.mutate({ channelId, subscribe: !subscribed })}
	/>
{/if}
