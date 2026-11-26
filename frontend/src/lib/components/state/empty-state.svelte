<script lang="ts">
	import type { Component, Snippet } from 'svelte';
	import { cn } from '#lib/utils.js';

	let {
		icon: Icon,
		title,
		description,
		headingLevel = 2,
		actions,
		class: className
	}: {
		icon?: Component<{ class?: string }>;
		title: string;
		description?: string;
		headingLevel?: 2 | 3;
		actions?: Snippet;
		class?: string;
	} = $props();
</script>

<div
	class={cn(
		'flex flex-col items-center justify-center gap-4 rounded-xl border border-dashed px-6 py-14 text-center',
		className
	)}
>
	{#if Icon}
		<div
			class="flex size-11 items-center justify-center rounded-full bg-muted text-muted-foreground"
		>
			<Icon class="size-5" />
		</div>
	{/if}
	<div class="flex max-w-sm flex-col gap-1.5">
		<svelte:element this={`h${headingLevel}`} class="text-base font-semibold tracking-tight">
			{title}
		</svelte:element>
		{#if description}
			<p class="text-sm text-pretty text-muted-foreground">{description}</p>
		{/if}
	</div>
	{#if actions}
		<div class="flex flex-wrap justify-center gap-2">{@render actions()}</div>
	{/if}
</div>
