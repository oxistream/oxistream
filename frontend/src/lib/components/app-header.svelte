<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import LanguageSwitch from './language-switch.svelte';
	import ThemeSwitch from './theme-switch.svelte';

	let query = $state('');

	function search(event: SubmitEvent) {
		event.preventDefault();
		const q = query.trim();
		if (!q) return;
		const params = new URLSearchParams({ search_query: q });
		void goto(resolve(`/results?${params.toString()}`));
	}
</script>

<header class="flex h-14 shrink-0 items-center gap-2 border-b px-4">
	<Sidebar.Trigger class="-ms-1" />
	<Separator orientation="vertical" class="me-2 data-vertical:h-4 data-vertical:self-auto" />
	<form role="search" class="relative w-full max-w-md" onsubmit={search}>
		<SearchIcon
			class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
		/>
		<Input
			type="search"
			bind:value={query}
			placeholder={m.search_placeholder()}
			aria-label={m.search_label()}
			class="ps-8"
		/>
	</form>
	<div class="ms-auto flex items-center gap-1">
		<LanguageSwitch />
		<ThemeSwitch />
	</div>
</header>
