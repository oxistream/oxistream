<script lang="ts">
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
</script>

<div class="@container flex flex-col gap-5" aria-hidden="true">
	<Skeleton class="aspect-[6.2/1] w-full rounded-xl" />
	<div class="flex items-start gap-4 @xl:gap-5">
		<Skeleton class="size-16 shrink-0 rounded-full @xl:size-24" />
		<div class="flex flex-1 flex-col gap-2.5 @xl:pt-2">
			<Skeleton class="h-6 w-48" />
			<Skeleton class="h-3.5 w-64 max-w-full" />
			<Skeleton class="h-3.5 w-full max-w-md" />
		</div>
	</div>
</div>
