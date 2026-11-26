<script lang="ts">
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
</script>

<div class="flex items-center gap-3 px-2 py-2" aria-hidden="true">
	<Skeleton class="size-10 shrink-0 rounded-full" />
	<div class="flex flex-1 flex-col gap-2">
		<Skeleton class="h-3.5 w-40 max-w-2/3" />
		<Skeleton class="h-3 w-28 max-w-1/2" />
	</div>
	<Skeleton class="h-8 w-24 rounded-md" />
</div>
