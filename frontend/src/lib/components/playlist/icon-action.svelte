<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonProps } from '#lib/components/ui/button/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';

	let {
		label,
		tooltip = label,
		children,
		...rest
	}: Omit<ButtonProps, 'children'> & {
		/** Accessible name, and the tooltip unless one is given. */
		label: string;
		tooltip?: string;
		children: Snippet;
	} = $props();
</script>

<Tooltip.Provider delayDuration={300}>
	<Tooltip.Root>
		<Tooltip.Trigger>
			{#snippet child({ props })}
				<Button variant="outline" size="icon" {...props} {...rest} aria-label={label}>
					{@render children()}
				</Button>
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content>{tooltip}</Tooltip.Content>
	</Tooltip.Root>
</Tooltip.Provider>
