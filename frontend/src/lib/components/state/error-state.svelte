<script lang="ts">
	import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
	import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		title = m.error_load_title(),
		description = m.error_load_body(),
		onretry,
		class: className
	}: {
		title?: string;
		description?: string;
		onretry?: () => void;
		class?: string;
	} = $props();
</script>

<div
	role="alert"
	class={cn(
		'flex flex-col items-center justify-center gap-4 rounded-xl border border-destructive/25 bg-destructive/5 px-6 py-14 text-center',
		className
	)}
>
	<div
		class="flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive"
	>
		<CircleAlertIcon class="size-5" />
	</div>
	<div class="flex max-w-sm flex-col gap-1.5">
		<p class="text-base font-semibold tracking-tight">{title}</p>
		<p class="text-sm text-pretty text-muted-foreground">{description}</p>
	</div>
	{#if onretry}
		<Button variant="outline" onclick={onretry}>
			<RotateCwIcon />
			{m.action_retry()}
		</Button>
	{/if}
</div>
