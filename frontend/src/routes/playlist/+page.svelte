<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PageHeader from '#lib/components/page-header.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { m } from '#lib/paraglide/messages.js';
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.search_playlist()} />
	<EmptyState icon={ListVideoIcon} title={m.search_playlist_soon()} />
</div>
