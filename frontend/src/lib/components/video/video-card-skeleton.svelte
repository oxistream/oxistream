<script lang="ts">
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';

	let { showChannel = true }: { showChannel?: boolean } = $props();
</script>

<div class="flex flex-col gap-3" aria-hidden="true">
	<Skeleton class="aspect-video w-full rounded-lg" />
	<div class="flex gap-3">
		{#if showChannel}
			<Skeleton class="mt-0.5 size-8 shrink-0 rounded-full" />
		{/if}
		<div class="flex flex-1 flex-col gap-2 pt-0.5">
			<Skeleton class="h-3.5 w-11/12" />
			<Skeleton class="h-3.5 w-2/3" />
			<Skeleton class="mt-1 h-3 w-1/2" />
		</div>
	</div>
</div>
