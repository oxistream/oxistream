<script lang="ts">
	import type { Snippet } from 'svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { cn } from '#lib/utils.js';

	let {
		title,
		description,
		documentTitle = pageTitle(title),
		actions,
		class: className
	}: {
		title: string;
		description?: string;
		/** Defaults to "<title> · Oxistream". */
		documentTitle?: string;
		actions?: Snippet;
		class?: string;
	} = $props();
</script>

<svelte:head>
	<title>{documentTitle}</title>
</svelte:head>

<div class={cn('flex flex-wrap items-end justify-between gap-x-4 gap-y-2', className)}>
	<div class="flex min-w-0 flex-col gap-1">
		<h1 class="text-xl font-semibold tracking-tight">{title}</h1>
		{#if description}
			<p class="text-sm text-muted-foreground">{description}</p>
		{/if}
	</div>
	{#if actions}
		<div class="flex items-center gap-2">{@render actions()}</div>
	{/if}
</div>
