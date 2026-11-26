<script lang="ts">
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { createSearchQuery } from '#lib/api/search.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { videoIdFromInput } from '#lib/together/sync.js';
	import { m } from '#lib/paraglide/messages.js';

	const MAX_RESULTS = 6;

	let {
		onadd,
		error
	}: {
		onadd: (videoId: string) => void;
		/** A message about the last attempt, e.g. from the server. */
		error?: string | null;
	} = $props();

	const uid = $props.id();
	let value = $state('');
	let term = $state('');

	const search = createSearchQuery(
		() => term,
		() => 'videos'
	);
	const results = $derived(
		(search.data?.pages[0]?.items ?? [])
			.flatMap((item) => (item.type === 'video' && !item.video.isLive ? [item.video] : []))
			.slice(0, MAX_RESULTS)
	);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const id = videoIdFromInput(value);
		if (id) {
			onadd(id);
			value = '';
			term = '';
		} else {
			term = value.trim();
		}
	}

	function add(id: string) {
		onadd(id);
		value = '';
		term = '';
	}
</script>

<form class="flex flex-col gap-2" onsubmit={submit} data-testid="room-add">
	<label for="{uid}-input" class="sr-only">{m.together_add_label()}</label>
	<div class="flex gap-2">
		<Input
			id="{uid}-input"
			bind:value
			placeholder={m.together_add_placeholder()}
			autocomplete="off"
			aria-invalid={error ? true : undefined}
			aria-describedby={error ? `${uid}-error` : undefined}
			class="h-8 flex-1"
		/>
		<Button type="submit" size="sm" disabled={value.trim().length === 0}>
			{#if value.trim() && !videoIdFromInput(value)}<SearchIcon />{:else}<PlusIcon />{/if}
			{m.together_add()}
		</Button>
	</div>
	{#if error}
		<p id="{uid}-error" class="text-xs font-medium text-destructive" role="alert">{error}</p>
	{/if}
	{#if term}
		{#if search.isPending}
			<div class="flex flex-col gap-2" role="status" aria-label={m.together_search_loading()}>
				{#each { length: 3 }, i (i)}
					<div class="flex gap-2">
						<Skeleton class="aspect-video w-20 shrink-0 rounded-md" />
						<Skeleton class="h-3.5 flex-1" />
					</div>
				{/each}
			</div>
		{:else if results.length === 0}
			<p class="text-xs text-muted-foreground">{m.together_search_empty()}</p>
		{:else}
			<ul class="flex flex-col gap-0.5" aria-label={m.together_add_label()}>
				{#each results as video (video.id)}
					<li>
						<button
							type="button"
							class="flex w-full items-center gap-2.5 rounded-lg p-1.5 text-start outline-ring hover:bg-muted/60 focus-visible:outline-2"
							aria-label={m.together_add_result({ title: video.title })}
							onclick={() => add(video.id)}
						>
							<VideoThumbnail
								src={video.thumbnailUrl}
								durationSeconds={video.durationSeconds}
								class="w-20 shrink-0 rounded-md"
							/>
							<span class="flex min-w-0 flex-1 flex-col">
								<span class="line-clamp-2 text-xs leading-4 font-medium">{video.title}</span>
								<span class="truncate text-[0.6875rem] text-muted-foreground">
									{video.channel.name}
								</span>
							</span>
							<PlusIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</form>
