<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { parseChannelTab } from '#lib/channel-tabs.js';
	import ChannelView from '#lib/components/channel/channel-view.svelte';

	const id = $derived(page.params.id ?? '');
	const tab = $derived(parseChannelTab(page.params.tab));
</script>

{#key id}
	<ChannelView {id} base={resolve('/channel/[id]', { id })} {tab} />
{/key}
