<script lang="ts">
	import { resolve } from '$app/paths';
	import { createContinueWatchingQuery, createDismissContinueMutation } from '#lib/api/history.js';
	import { rowState } from '#lib/home/sections.js';
	import { m } from '#lib/paraglide/messages.js';
	import HomeRow from './home-row.svelte';

	const query = createContinueWatchingQuery();
	const dismiss = createDismissContinueMutation();
	const state = $derived(rowState({ isPending: query.isPending, count: query.data?.length ?? 0 }));
</script>

{#if state !== 'hidden'}
	<HomeRow
		title={m.home_continue_watching()}
		href={resolve('/feed/history')}
		videos={query.data ?? []}
		loading={state === 'loading'}
		removal={{ label: m.action_remove(), onremove: (video) => dismiss.mutate(video.id) }}
	/>
{/if}
