<script lang="ts">
	import GraduationCapIcon from '@lucide/svelte/icons/graduation-cap';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();
</script>

<span
	class={cn(
		'inline-flex w-fit shrink-0 items-center gap-1 rounded-sm border bg-muted px-1.5 py-px text-xs font-medium text-foreground',
		className
	)}
>
	<GraduationCapIcon class="size-3.5 shrink-0 text-primary" aria-hidden="true" />
	{m.playlist_course()}
</span>
