<script lang="ts">
	import ConstructionIcon from '@lucide/svelte/icons/construction';
	import { m } from '#lib/paraglide/messages.js';
	import PageHeader from './page-header.svelte';
	import EmptyState from './state/empty-state.svelte';

	let { title }: { title: string } = $props();
</script>

<div class="flex flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader {title} />
	<EmptyState icon={ConstructionIcon} title={m.page_placeholder()} />
</div>
