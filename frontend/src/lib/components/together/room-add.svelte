<script lang="ts">
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { uniqueBy } from '#lib/api/mapping.js';
	import { createSearchQuery } from '#lib/api/search.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { videoIdFromInput } from '#lib/together/sync.js';
	import type { VideoSummary } from '#lib/video.js';
	import { m } from '#lib/paraglide/messages.js';
	import RoomSearchResults from './room-search-results.svelte';

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
	/** Added from the current results, so they are marked instead of added twice by accident. */
	let added = $state<string[]>([]);

	const search = createSearchQuery(
		() => term,
		() => 'videos'
	);
	const playable = (video: VideoSummary) => !video.isLive && !video.isShort && !video.isUpcoming;
	const results = $derived(
		uniqueBy(
			(search.data?.pages ?? []).flatMap((page) =>
				page.items.flatMap((item) =>
					item.type === 'video' && playable(item.video) ? [item.video] : []
				)
			),
			(video) => video.id
		)
	);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const id = videoIdFromInput(value);
		if (id) {
			onadd(id);
			clear();
		} else {
			term = value.trim();
			added = [];
		}
	}

	function add(id: string) {
		onadd(id);
		added = [...added, id];
	}

	function clear() {
		value = '';
		term = '';
		added = [];
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && term) {
			event.preventDefault();
			clear();
		}
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
			onkeydown={keydown}
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
		<RoomSearchResults
			{results}
			{added}
			pending={search.isPending}
			failed={search.isError}
			hasMore={search.hasNextPage}
			loadingMore={search.isFetchingNextPage}
			moreFailed={search.isFetchNextPageError}
			onadd={add}
			onmore={() => void search.fetchNextPage()}
			onclear={clear}
		/>
	{/if}
</form>
