<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';

	let { title }: { title: string } = $props();
</script>

<svelte:head>
	<title>{title} · Oxistream</title>
</svelte:head>

<section class="flex flex-1 flex-col gap-4 p-4">
	<h1 class="text-xl font-semibold tracking-tight">{title}</h1>
	<p class="text-sm text-muted-foreground">{m.page_placeholder()}</p>
</section>
