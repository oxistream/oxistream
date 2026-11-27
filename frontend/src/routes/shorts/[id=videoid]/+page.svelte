<script lang="ts">
	import { page } from '$app/state';
	import ShortsViewer from '#lib/components/shorts/shorts-viewer.svelte';
	import { session } from '#lib/session.svelte.js';

	const id = $derived(page.params.id ?? '');
	const param = $derived(page.url.searchParams.get('feed'));
	// Anonymous users have no subscriptions, so their subscription feed falls back to discovery.
	const feed = $derived(
		param === 'discover' || (param === 'subscriptions' && !session.user)
			? 'discover'
			: param === 'subscriptions'
				? 'subscriptions'
				: undefined
	);
</script>

{#if !param || session.ready}
	{#key `${id}:${feed}`}
		<ShortsViewer openedId={id} {feed} />
	{/key}
{/if}
