<script lang="ts">
	import { resolve } from '$app/paths';
	import LogoMark from '#lib/components/brand/logo-mark.svelte';
	import SearchBox from '#lib/components/search/search-box.svelte';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { useSidebar } from '#lib/components/ui/sidebar/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import LanguageSwitch from './language-switch.svelte';
	import ThemeSwitch from './theme-switch.svelte';

	const sidebar = useSidebar();

	let input = $state<HTMLInputElement | null>(null);

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
	class="sticky top-0 z-20 flex h-14 shrink-0 items-center gap-2 border-b bg-background/90 px-(--page-gutter) backdrop-blur-md"
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
	<SearchBox bind:input />
	<div class="ms-auto flex shrink-0 items-center gap-0.5">
		<LanguageSwitch />
		<ThemeSwitch />
	</div>
</header>
