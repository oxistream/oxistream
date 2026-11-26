<script lang="ts">
	import { page } from '$app/state';
	import PagePlaceholder from '#lib/components/page-placeholder.svelte';
	import { m } from '#lib/paraglide/messages.js';

	const query = $derived(page.url.searchParams.get('search_query') ?? '');
</script>

<PagePlaceholder title={m.search_results_title({ query })} />
