<script lang="ts" generics="T extends string">
	import { cn } from '#lib/utils.js';

	let {
		name,
		legend,
		description,
		options,
		value,
		onchange,
		class: className
	}: {
		name: string;
		legend: string;
		description?: string;
		options: { value: T; label: string; lang?: string }[];
		value: T;
		onchange: (value: T) => void;
		class?: string;
	} = $props();

	const descriptionId = $derived(`${name}-description`);
</script>

<fieldset
	class={cn('min-w-0', className)}
	aria-describedby={description ? descriptionId : undefined}
>
	<legend class="text-sm font-medium">{legend}</legend>
	{#if description}
		<p id={descriptionId} class="mt-1 text-sm text-muted-foreground">{description}</p>
	{/if}
	<div class="mt-3 inline-flex w-fit flex-wrap gap-1 rounded-lg bg-muted p-1">
		{#each options as option (option.value)}
			<label
				lang={option.lang}
				class="relative inline-flex h-8 cursor-pointer items-center rounded-md px-3 text-sm font-medium text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground has-checked:bg-background has-checked:text-foreground has-checked:shadow-xs has-focus-visible:outline-2 has-focus-visible:outline-offset-2 has-focus-visible:outline-ring has-focus-visible:outline-solid"
			>
				<input
					type="radio"
					{name}
					value={option.value}
					checked={option.value === value}
					onchange={() => onchange(option.value)}
					class="sr-only"
				/>
				{option.label}
			</label>
		{/each}
	</div>
</fieldset>
