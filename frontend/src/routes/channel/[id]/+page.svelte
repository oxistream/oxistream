<script lang="ts">
	import { page } from '$app/state';
	import ChannelView from '#lib/components/channel/channel-view.svelte';

	const id = $derived(page.params.id ?? '');
</script>

{#key id}
	<ChannelView {id} />
{/key}
