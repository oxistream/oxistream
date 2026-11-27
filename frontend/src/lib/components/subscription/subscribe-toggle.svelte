<script lang="ts">
	import BellCheckIcon from '@lucide/svelte/icons/bell-check';
	import BellPlusIcon from '@lucide/svelte/icons/bell-plus';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		subscribed = false,
		loading = false,
		size = 'default',
		onclick,
		class: className
	}: {
		subscribed?: boolean;
		/** The subscription state is not known yet. */
		loading?: boolean;
		size?: 'default' | 'sm';
		onclick?: () => void;
		class?: string;
	} = $props();

	const base = $derived(cn('rounded-full', size === 'sm' ? 'px-3.5' : 'px-5', className));
</script>

<Button
	{size}
	variant={subscribed ? 'secondary' : 'default'}
	class={cn(base, loading && 'cursor-wait')}
	aria-busy={loading || undefined}
	disabled={loading}
	data-testid="subscribe-button"
	data-subscribed={subscribed}
	{onclick}
>
	{#if subscribed}
		<BellCheckIcon class="text-primary" />
		{m.subscription_subscribed()}
	{:else}
		<BellPlusIcon />
		{m.subscription_subscribe()}
	{/if}
</Button>
