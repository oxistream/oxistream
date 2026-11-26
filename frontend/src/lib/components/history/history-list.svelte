<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';
	import type { HistoryDay } from '#lib/api/history-list.js';
	import ResultSkeleton from '#lib/components/search/result-skeleton.svelte';
	import VideoResult from '#lib/components/search/video-result.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		days = [],
		loading = false,
		onremove,
		now,
		class: className
	}: {
		days?: HistoryDay[];
		loading?: boolean;
		onremove?: (videoId: string) => void;
		/** Reference time for relative dates; fixed in stories. */
		now?: Date;
		class?: string;
	} = $props();
</script>

<div class={cn('@container', className)}>
	{#if loading}
		<div class="flex flex-col gap-6" role="status" aria-label={m.history_loading()}>
			{#each { length: 5 }, i (i)}<ResultSkeleton />{/each}
		</div>
	{:else}
		<div class="flex flex-col gap-8">
			{#each days as day (day.key)}
				<section aria-labelledby="history-day-{day.key}" class="flex flex-col gap-4">
					<h2 id="history-day-{day.key}" class="text-base font-semibold tracking-tight">
						{day.label}
					</h2>
					<ul class="flex flex-col gap-6 @xl:gap-5" aria-label={day.label}>
						{#each day.videos as video (video.id)}
							<li class="flex min-w-0 items-start gap-2">
								<VideoResult {video} {now} actions={false} class="flex-1" />
								{#if onremove}
									<Button
										variant="ghost"
										size="icon-sm"
										class="shrink-0 text-muted-foreground"
										title={m.history_remove_short()}
										aria-label={m.history_remove({ title: video.title })}
										onclick={() => onremove(video.id)}
									>
										<XIcon />
									</Button>
								{/if}
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		</div>
	{/if}
</div>
