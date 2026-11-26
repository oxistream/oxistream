<script lang="ts">
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
</script>

<div
	class="grid gap-3 @xl:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] @xl:gap-4 @4xl:grid-cols-[22rem_minmax(0,1fr)]"
	aria-hidden="true"
>
	<Skeleton class="aspect-video w-full rounded-lg" />
	<div class="flex flex-col gap-2 @xl:pt-1">
		<Skeleton class="h-4 w-11/12" />
		<Skeleton class="h-4 w-2/3" />
		<Skeleton class="mt-1 h-3 w-1/3" />
		<div class="mt-1 flex items-center gap-2">
			<Skeleton class="size-6 rounded-full" />
			<Skeleton class="h-3 w-28" />
		</div>
		<Skeleton class="mt-1 hidden h-3 w-4/5 @xl:block" />
	</div>
</div>
