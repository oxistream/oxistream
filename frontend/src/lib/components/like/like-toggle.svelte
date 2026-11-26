<script lang="ts">
	import ThumbsUpIcon from '@lucide/svelte/icons/thumbs-up';
	import { Button } from '#lib/components/ui/button/index.js';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		liked = false,
		loading = false,
		signInHref,
		count,
		variant = 'default',
		onclick,
		class: className
	}: {
		liked?: boolean;
		/** The like state is not known yet. */
		loading?: boolean;
		/** Set for anonymous users: the button leads to sign-in instead. */
		signInHref?: string;
		/** YouTube's like count, shown instead of the label. Our likes never change it. */
		count?: number | null;
		/** `overlay` is an icon button for use over video, as in the shorts player. */
		variant?: 'default' | 'overlay';
		onclick?: () => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	const overlay = $derived(variant === 'overlay');
	const counted = $derived(count != null ? formatCompactNumber(count, locale) : undefined);
	const title = $derived(
		signInHref ? m.like_sign_in() : liked ? m.like_remove() : `${m.like()} · ${m.like_hint()}`
	);
	const base = $derived(
		overlay
			? cn(
					'rounded-full bg-black/35 text-white backdrop-blur-sm hover:bg-white/20 hover:text-white',
					className
				)
			: cn('rounded-full px-3.5', className)
	);
</script>

{#snippet content()}
	<ThumbsUpIcon
		class={cn(
			'transition-colors duration-(--motion-in) ease-ui',
			liked && 'fill-primary text-primary'
		)}
		aria-hidden="true"
	/>
	{#if overlay}
		<span class="sr-only">{m.like()}</span>
	{:else if counted}
		<span class="sr-only">{`${m.like()}, `}</span><span class="tabular">{counted}</span>
	{:else}
		{m.like()}
	{/if}
{/snippet}

{#if signInHref}
	<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved by authHref -->
	<Button
		href={signInHref}
		variant={overlay ? 'ghost' : 'secondary'}
		size={overlay ? 'icon-lg' : 'sm'}
		class={base}
		{title}
		data-testid="like-button"
	>
		{@render content()}
	</Button>
{:else}
	<Button
		variant={overlay ? 'ghost' : 'secondary'}
		size={overlay ? 'icon-lg' : 'sm'}
		class={cn(base, loading && 'cursor-wait')}
		aria-pressed={liked}
		aria-busy={loading || undefined}
		disabled={loading}
		{title}
		data-testid="like-button"
		data-liked={liked}
		{onclick}
	>
		{@render content()}
	</Button>
{/if}
