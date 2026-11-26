<script lang="ts">
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();
</script>

<div class={cn('flex flex-col gap-3', className)} aria-hidden="true">
	<Skeleton class="h-6 w-4/5" />
	<Skeleton class="h-6 w-2/5 sm:hidden" />
	<div class="flex items-center gap-3">
		<Skeleton class="size-10 shrink-0 rounded-full" />
		<div class="flex flex-col gap-1.5">
			<Skeleton class="h-3.5 w-32" />
			<Skeleton class="h-3 w-24" />
		</div>
		<Skeleton class="ms-1 h-8 w-24 rounded-full" />
	</div>
	<Skeleton class="h-28 w-full rounded-xl" />
</div>
