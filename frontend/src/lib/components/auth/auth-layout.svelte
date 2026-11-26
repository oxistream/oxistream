<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import Wordmark from '#lib/components/brand/wordmark.svelte';
	import LanguageSwitch from '#lib/components/language-switch.svelte';
	import ThemeSwitch from '#lib/components/theme-switch.svelte';

	let { children }: { children: Snippet } = $props();
</script>

<div class="flex min-h-svh flex-col">
	<header class="flex h-14 items-center justify-between px-(--page-gutter)">
		<a href={resolve('/')} class="rounded-md py-1">
			<Wordmark />
		</a>
		<div class="flex items-center gap-0.5">
			<LanguageSwitch />
			<ThemeSwitch />
		</div>
	</header>
	<main
		id="main"
		tabindex="-1"
		class="flex flex-1 items-start justify-center px-(--page-gutter) pt-[min(8vh,4rem)] pb-16 outline-none"
	>
		{@render children()}
	</main>
</div>
