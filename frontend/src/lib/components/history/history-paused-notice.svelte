<script lang="ts">
	import PauseIcon from '@lucide/svelte/icons/pause';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();
</script>

<div
	class={cn('flex items-start gap-3 rounded-xl border bg-muted/40 px-4 py-3', className)}
	role="status"
>
	<PauseIcon class="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
	<div class="flex min-w-0 flex-col gap-0.5">
		<p class="text-sm font-medium">{m.history_paused_title()}</p>
		<p class="text-sm text-muted-foreground">{m.history_paused_body()}</p>
	</div>
</div>
