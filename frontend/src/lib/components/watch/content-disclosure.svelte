<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import WandSparklesIcon from '@lucide/svelte/icons/wand-sparkles';
	import type { ContentDisclosure } from '#lib/api/video.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		disclosures,
		variant = 'default',
		class: className
	}: {
		disclosures?: readonly ContentDisclosure[];
		/** `overlay` sits on top of a short. */
		variant?: 'default' | 'overlay';
		class?: string;
	} = $props();

	// Auto-dubs show in the player's audio menu instead.
	const disclosure = $derived(disclosures?.find((d) => d.kind !== 'auto_dubbed'));
	const label = $derived(
		disclosure?.kind === 'altered_or_synthetic'
			? m.disclosure_altered()
			: (disclosure?.title ?? m.disclosure_altered())
	);
	let expanded = $state(false);
	const uid = $props.id();
	const overlay = $derived(variant === 'overlay');
</script>

{#if disclosure}
	<div
		class={cn('flex min-w-0 flex-col items-start gap-1.5', className)}
		data-testid="content-disclosure"
	>
		<button
			type="button"
			class={cn(
				'inline-flex max-w-full items-center gap-1.5 rounded-full py-1 ps-2 pe-2.5 text-xs font-medium transition-colors duration-(--motion-in) ease-ui focus-visible:ring-2 focus-visible:outline-none',
				overlay
					? 'pointer-events-auto bg-black/40 text-white backdrop-blur-sm hover:bg-white/20 focus-visible:ring-white'
					: 'bg-muted text-muted-foreground hover:text-foreground focus-visible:ring-ring'
			)}
			aria-expanded={expanded}
			aria-controls="{uid}-details"
			onclick={() => (expanded = !expanded)}
		>
			<WandSparklesIcon class="size-3.5 shrink-0" aria-hidden="true" />
			<span class="truncate">{label}</span>
			<ChevronDownIcon
				class={cn('size-3.5 shrink-0 transition-transform', expanded && 'rotate-180')}
				aria-hidden="true"
			/>
		</button>
		<div
			id="{uid}-details"
			hidden={!expanded}
			class={cn(
				'max-w-prose rounded-lg px-3 py-2 text-xs leading-5 text-pretty',
				overlay
					? 'pointer-events-auto bg-black/55 text-white/90 backdrop-blur-sm'
					: 'border bg-card text-muted-foreground'
			)}
		>
			{#if disclosure.title}
				<p class={overlay ? 'font-medium text-white' : 'font-medium text-foreground'}>
					{disclosure.title}
				</p>
			{/if}
			<p>
				{disclosure.text}
			</p>
			{#if disclosure.attribution || disclosure.help_url}
				<p class="mt-1 flex flex-wrap gap-x-3">
					{#if disclosure.attribution}<span>{disclosure.attribution}</span>{/if}
					{#if disclosure.help_url}
						<!-- eslint-disable svelte/no-navigation-without-resolve -- external page -->
						<a
							href={disclosure.help_url}
							target="_blank"
							rel="noopener noreferrer"
							class={cn(
								'underline underline-offset-2',
								overlay ? 'hover:text-white' : 'hover:text-foreground'
							)}
						>
							{m.disclosure_learn_more()}
						</a>
						<!-- eslint-enable svelte/no-navigation-without-resolve -->
					{/if}
				</p>
			{/if}
		</div>
	</div>
{/if}
