<script lang="ts">
	import BadgeCheckIcon from '@lucide/svelte/icons/badge-check';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();
</script>

<span
	role="img"
	aria-label={m.search_verified()}
	title={m.search_verified()}
	class={cn('inline-flex shrink-0 text-muted-foreground', className)}
>
	<BadgeCheckIcon class="size-[1em]" aria-hidden="true" />
</span>
