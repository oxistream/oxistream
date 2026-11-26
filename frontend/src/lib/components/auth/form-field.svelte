<script lang="ts">
	import type { FullAutoFill } from 'svelte/elements';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';

	let {
		id,
		name,
		label,
		type = 'text',
		autocomplete,
		value = $bindable(''),
		error,
		hint
	}: {
		id: string;
		name: string;
		label: string;
		type?: 'text' | 'password' | 'email';
		autocomplete?: FullAutoFill;
		value?: string;
		error?: string;
		hint?: string;
	} = $props();

	const describedBy = $derived(
		[hint && !error ? `${id}-hint` : '', error ? `${id}-error` : ''].filter(Boolean).join(' ') ||
			undefined
	);
</script>

<div class="flex flex-col gap-2">
	<Label for={id}>{label}</Label>
	<Input
		{id}
		{name}
		{type}
		{autocomplete}
		autocapitalize={type === 'password' ? undefined : 'none'}
		spellcheck={false}
		bind:value
		aria-invalid={error ? true : undefined}
		aria-describedby={describedBy}
		class="h-10 text-base md:text-sm"
	/>
	{#if hint && !error}
		<p id="{id}-hint" class="text-xs text-muted-foreground">{hint}</p>
	{/if}
	{#if error}
		<p id="{id}-error" class="text-xs font-medium text-destructive">{error}</p>
	{/if}
</div>
