<script lang="ts">
	import { page } from '$app/state';
	import ShortsViewer from '#lib/components/shorts/shorts-viewer.svelte';
	import { session } from '#lib/session.svelte.js';

	// Anonymous users have no subscriptions, so they always get discovery.
	const feed = $derived(
		page.url.searchParams.get('feed') === 'subscriptions' && session.user
			? 'subscriptions'
			: 'discover'
	);
</script>

{#if session.ready}
	{#key feed}
		<ShortsViewer {feed} />
	{/key}
{/if}
