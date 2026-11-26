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
		id?: string;
		/** Omitted when the page heading already names the section. */
		title?: string;
		description?: string;
		children: Snippet;
		class?: string;
	} = $props();
</script>

<section
	{id}
	aria-labelledby={title && id ? `${id}-heading` : undefined}
	class={cn('rounded-xl border bg-card', className)}
>
	{#if title}
		<header class="flex flex-col gap-1 border-b px-5 py-4">
			<h2 id={id ? `${id}-heading` : undefined} class="text-base font-semibold tracking-tight">
				{title}
			</h2>
			{#if description}
				<p class="text-sm text-muted-foreground">{description}</p>
			{/if}
		</header>
	{/if}
	<div class="flex flex-col divide-y">
		{@render children()}
	</div>
</section>
