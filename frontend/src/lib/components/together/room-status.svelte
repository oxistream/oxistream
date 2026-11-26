<script lang="ts">
	import type { ConnectionStatus } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { status, class: className }: { status: ConnectionStatus; class?: string } = $props();

	const label = $derived(
		status === 'open'
			? m.together_status_open()
			: status === 'reconnecting'
				? m.together_status_reconnecting()
				: m.together_status_connecting()
	);
</script>

<span
	class={cn(
		'inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium text-muted-foreground',
		className
	)}
	role="status"
	data-status={status}
	data-testid="room-status"
>
	<span
		class={cn(
			'size-1.5 rounded-full',
			status === 'open' ? 'bg-emerald-500' : 'animate-pulse bg-amber-500'
		)}
		aria-hidden="true"
	></span>
	{label}
</span>
