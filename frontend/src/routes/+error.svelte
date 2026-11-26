<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { m } from '#lib/paraglide/messages.js';

	const title = $derived(page.status === 404 ? m.error_not_found() : m.error_generic());
</script>

<svelte:head>
	<title>{title} · Oxistream</title>
</svelte:head>

<section class="flex flex-1 flex-col items-start gap-2 p-4">
	<p class="text-sm text-muted-foreground">{page.status}</p>
	<h1 class="text-xl font-semibold tracking-tight">{title}</h1>
	<a href={resolve('/')} class="text-sm text-primary underline-offset-4 hover:underline">
		{m.error_back_home()}
	</a>
</section>
