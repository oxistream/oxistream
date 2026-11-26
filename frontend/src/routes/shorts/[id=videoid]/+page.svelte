<script lang="ts">
	import { page } from '$app/state';
	import ShortsViewer from '#lib/components/shorts/shorts-viewer.svelte';

	const id = $derived(page.params.id ?? '');
</script>

{#key id}
	<ShortsViewer openedId={id} />
{/key}
