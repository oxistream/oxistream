<script lang="ts">
	import type { ConnectionStatus } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { status, class: className }: { status: ConnectionStatus; class?: string } = $props();

	const label = $derived(
		status === 'open'
			? m.party_status_open()
			: status === 'reconnecting'
				? m.party_status_reconnecting()
				: m.party_status_connecting()
	);
</script>

<!-- Connected is the normal case: only a dot then, the label while connecting. -->
<span
	class={cn(
		'inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground',
		className
	)}
	role="status"
	title={label}
	data-status={status}
	data-testid="party-status"
>
	<span
		class={cn(
			'size-2 rounded-full',
			status === 'open' ? 'bg-emerald-500' : 'animate-pulse bg-amber-500 motion-reduce:animate-none'
		)}
		aria-hidden="true"
	></span>
	<span class={cn(status === 'open' && 'sr-only')}>{label}</span>
</span>
