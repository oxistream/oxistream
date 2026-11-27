<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { VideoSummary } from '#lib/video.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		results,
		pending = false,
		failed = false,
		hasMore = false,
		loadingMore = false,
		moreFailed = false,
		added = [],
		onadd,
		onmore,
		onclear
	}: {
		results: VideoSummary[];
		/** The first page is loading. */
		pending?: boolean;
		failed?: boolean;
		hasMore?: boolean;
		loadingMore?: boolean;
		moreFailed?: boolean;
		/** IDs added from these results, shown as done. */
		added?: string[];
		onadd: (videoId: string) => void;
		onmore: () => void;
		onclear: () => void;
	} = $props();
</script>

<div class="flex items-center justify-between gap-2">
	<span class="text-xs font-medium text-muted-foreground">{m.together_search_results()}</span>
	<Button
		type="button"
		variant="ghost"
		size="icon-sm"
		class="size-6 rounded-full text-muted-foreground"
		aria-label={m.together_search_clear()}
		title={m.together_search_clear()}
		onclick={onclear}
	>
		<XIcon />
	</Button>
</div>
{#if pending}
	<div class="flex flex-col gap-2" role="status" aria-label={m.together_search_loading()}>
		{#each { length: 4 }, i (i)}
			<div class="flex gap-2.5 p-1.5">
				<Skeleton class="aspect-video w-28 shrink-0 rounded-md" />
				<div class="flex flex-1 flex-col gap-1.5 py-0.5">
					<Skeleton class="h-3.5 w-full" />
					<Skeleton class="h-3 w-2/3" />
				</div>
			</div>
		{/each}
	</div>
{:else if failed && results.length === 0}
	<p class="text-xs text-muted-foreground" role="alert">{m.error_load_body()}</p>
{:else if results.length === 0 && !hasMore}
	<p class="text-xs text-muted-foreground">{m.together_search_empty()}</p>
{:else}
	<div class="-mx-1 max-h-[min(32rem,60svh)] overflow-y-auto overscroll-contain px-1">
		<ul class="flex flex-col gap-0.5" aria-label={m.together_search_results()}>
			{#each results as video (video.id)}
				{@const done = added.includes(video.id)}
				<li>
					<button
						type="button"
						class="flex w-full items-start gap-2.5 rounded-lg p-1.5 text-start outline-ring hover:bg-muted/60 focus-visible:outline-2 disabled:opacity-60"
						aria-label={m.together_add_result({ title: video.title })}
						disabled={done}
						onclick={() => onadd(video.id)}
					>
						<VideoThumbnail
							src={video.thumbnailUrl}
							durationSeconds={video.durationSeconds}
							class="w-28 shrink-0 rounded-md"
						/>
						<span class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="line-clamp-2 text-xs leading-4 font-medium">{video.title}</span>
							<span class="truncate text-[0.6875rem] text-muted-foreground">
								{video.channel.name}
							</span>
						</span>
						{#if done}
							<CheckIcon class="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
						{:else}
							<PlusIcon class="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
						{/if}
					</button>
				</li>
			{/each}
		</ul>
		<InfiniteLoader
			{hasMore}
			loading={loadingMore}
			failed={moreFailed}
			onload={onmore}
			class="min-h-10 py-2"
		/>
	</div>
{/if}
