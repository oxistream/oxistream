<script lang="ts">
	import BellCheckIcon from '@lucide/svelte/icons/bell-check';
	import BellPlusIcon from '@lucide/svelte/icons/bell-plus';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		subscribed = false,
		loading = false,
		signInHref,
		size = 'default',
		onclick,
		class: className
	}: {
		subscribed?: boolean;
		/** The subscription state is not known yet. */
		loading?: boolean;
		/** Set for anonymous users: the button leads to sign-in instead. */
		signInHref?: string;
		size?: 'default' | 'sm';
		onclick?: () => void;
		class?: string;
	} = $props();

	const base = $derived(cn(size === 'sm' && 'rounded-full px-3.5', className));
</script>

{#if signInHref}
	<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved by authHref -->
	<Button href={signInHref} {size} class={base} data-testid="subscribe-button">
		<BellPlusIcon />
		{m.subscription_subscribe()}
	</Button>
{:else}
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
{/if}
