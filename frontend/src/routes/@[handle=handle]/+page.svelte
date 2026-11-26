<script lang="ts">
	import { page } from '$app/state';
	import { createHandleQuery } from '#lib/api/channel.js';
	import ChannelView from '#lib/components/channel/channel-view.svelte';

	// The URL stays on /@handle, like on YouTube; the handle is only resolved to load the channel.
	const handle = $derived(page.params.handle ?? '');
	const resolved = createHandleQuery(() => handle);
</script>

{#key handle}
	<ChannelView
		id={resolved.data}
		resolving={resolved.isPending}
		resolveError={resolved.error ?? undefined}
		onretry={() => void resolved.refetch()}
	/>
{/key}
