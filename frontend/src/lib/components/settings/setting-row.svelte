<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '#lib/utils.js';

	let {
		id,
		label,
		description,
		control,
		children,
		stacked = false,
		class: className
	}: {
		/** Labels the control: its element must carry this id. */
		id?: string;
		label: string;
		description?: string;
		/** Shown beside the label, or below it when `stacked`. */
		control?: Snippet;
		/** Full-width content below the label. */
		children?: Snippet;
		stacked?: boolean;
		class?: string;
	} = $props();
</script>

<div class={cn('flex flex-col gap-3 px-5 py-4', className)}>
	<div
		class={cn(
			'flex gap-x-6 gap-y-3',
			stacked ? 'flex-col' : 'flex-col sm:flex-row sm:items-center sm:justify-between'
		)}
	>
		<div class="flex min-w-0 flex-col gap-1">
			{#if id}
				<label for={id} class="text-sm font-medium">{label}</label>
			{:else}
				<span class="text-sm font-medium">{label}</span>
			{/if}
			{#if description}
				<p id={id ? `${id}-description` : undefined} class="text-sm text-muted-foreground">
					{description}
				</p>
			{/if}
		</div>
		{#if control}
			<div class="shrink-0">{@render control()}</div>
		{/if}
	</div>
	{@render children?.()}
</div>
