<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import LogoMark from '#lib/components/brand/logo-mark.svelte';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { useSidebar } from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import LanguageSwitch from './language-switch.svelte';
	import ThemeSwitch from './theme-switch.svelte';

	const sidebar = useSidebar();

	let query = $state(page.url.searchParams.get('search_query') ?? '');
	let input = $state<HTMLInputElement | null>(null);

	function search(event: SubmitEvent) {
		event.preventDefault();
		const q = query.trim();
		if (!q) return;
		const params = new URLSearchParams({ search_query: q });
		void goto(resolve(`/results?${params.toString()}`));
	}

	function isEditable(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) return false;
		return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== '/' || event.ctrlKey || event.metaKey || event.altKey) return;
		if (event.defaultPrevented || isEditable(event.target)) return;
		event.preventDefault();
		input?.focus();
		input?.select();
	}
</script>

<svelte:window {onkeydown} />

<header
	class="sticky top-0 z-20 flex h-14 shrink-0 items-center gap-2 border-b bg-background/90 px-(--page-gutter) backdrop-blur-md md:rounded-t-xl"
>
	<Sidebar.Trigger
		class="-ms-1.5"
		aria-expanded={sidebar.isMobile ? sidebar.openMobile : sidebar.open}
	/>
	<a href={resolve('/')} class="flex size-8 items-center justify-center rounded-md md:hidden">
		<LogoMark class="size-6 text-primary" />
		<span class="sr-only">{m.nav_home()}</span>
	</a>
	<Separator
		orientation="vertical"
		class="me-1 hidden md:block data-vertical:h-4 data-vertical:self-auto"
	/>
	<form role="search" class="relative min-w-0 flex-1 md:max-w-md" onsubmit={search}>
		<SearchIcon
			class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
		/>
		<Input
			bind:ref={input}
			type="search"
			bind:value={query}
			placeholder={m.search_placeholder()}
			aria-label={m.search_label()}
			aria-keyshortcuts="/"
			enterkeyhint="search"
			data-testid="search-input"
			class="peer bg-muted/60 ps-8 md:pe-9 dark:bg-input/30"
		/>
		<kbd
			class="pointer-events-none absolute end-2 top-1/2 hidden h-5 -translate-y-1/2 items-center rounded border bg-background px-1.5 font-sans text-xs text-muted-foreground peer-focus:opacity-0 md:inline-flex"
			title={m.search_shortcut()}
			aria-hidden="true"
		>
			/
		</kbd>
	</form>
	<div class="ms-auto flex shrink-0 items-center gap-0.5">
		<LanguageSwitch />
		<ThemeSwitch />
	</div>
</header>
