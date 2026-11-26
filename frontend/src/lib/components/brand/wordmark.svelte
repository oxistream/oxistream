<script lang="ts">
	import { cn } from '#lib/utils.js';
	import LogoMark from './logo-mark.svelte';

	let { class: className }: { class?: string } = $props();
</script>

<span class={cn('inline-flex items-center gap-2 text-foreground', className)}>
	<LogoMark class="size-6 shrink-0 text-primary" />
	<span class="text-[1.0625rem] leading-none font-semibold tracking-[-0.02em]">Oxistream</span>
</span>
