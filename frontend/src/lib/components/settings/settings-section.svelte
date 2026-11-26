<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '#lib/utils.js';

	let {
		id,
		title,
		description,
		children,
		class: className
	}: {
		/** Anchor for the settings navigation. */
		id: string;
		title: string;
		description?: string;
		children: Snippet;
		class?: string;
	} = $props();
</script>

<section
	{id}
	aria-labelledby="{id}-heading"
	class={cn('scroll-mt-20 rounded-xl border bg-card', className)}
>
	<header class="flex flex-col gap-1 border-b px-5 py-4">
		<h2 id="{id}-heading" class="text-base font-semibold tracking-tight">{title}</h2>
		{#if description}
			<p class="text-sm text-muted-foreground">{description}</p>
		{/if}
	</header>
	<div class="flex flex-col divide-y">
		{@render children()}
	</div>
</section>
