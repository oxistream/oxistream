<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import Wordmark from '#lib/components/brand/wordmark.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';

	const title = $derived(page.status === 404 ? m.error_not_found() : m.error_generic());
</script>

<svelte:head>
	<title>{pageTitle(title)}</title>
</svelte:head>

<div class="flex min-h-svh flex-col">
	<header class="flex h-14 items-center px-(--page-gutter)">
		<a href={resolve('/')} class="rounded-md py-1">
			<Wordmark />
		</a>
	</header>
	<main
		id="main"
		tabindex="-1"
		class="flex flex-1 flex-col items-center justify-center gap-4 px-(--page-gutter) pb-24 text-center outline-none"
	>
		<p class="text-sm font-medium text-muted-foreground tabular">{page.status}</p>
		<h1 class="text-2xl font-semibold tracking-tight">{title}</h1>
		<Button href={resolve('/')} variant="outline">{m.error_back_home()}</Button>
	</main>
</div>
