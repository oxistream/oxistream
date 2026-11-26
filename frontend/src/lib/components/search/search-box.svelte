<script lang="ts">
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { createSuggestionsQuery, resolveYouTubeUrl } from '#lib/api/search.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import { parseYouTubeUrl, resolvedPath } from '#lib/youtube.js';

	let {
		input = $bindable(null),
		debounceMs = 180,
		class: className
	}: { input?: HTMLInputElement | null; debounceMs?: number; class?: string } = $props();

	const uid = $props.id();
	const listboxId = `${uid}-suggestions`;
	const MAX_SUGGESTIONS = 8;

	let query = $state('');
	let debounced = $state('');
	let focused = $state(false);
	let dismissed = $state(false);
	let active = $state(-1);
	let resolving = $state(false);

	// The box mirrors the query of the results page and empties on other pages.
	$effect(() => {
		const url = page.url;
		query = url.pathname.endsWith('/results') ? (url.searchParams.get('search_query') ?? '') : '';
		dismissed = true;
	});

	$effect(() => {
		const q = query.trim();
		const timer = setTimeout(() => (debounced = q), debounceMs);
		return () => clearTimeout(timer);
	});

	const isUrl = $derived(parseYouTubeUrl(query) !== null);
	const suggestionsQuery = createSuggestionsQuery(() => (isUrl ? '' : debounced));
	const suggestions = $derived(
		query.trim() && !isUrl ? (suggestionsQuery.data ?? []).slice(0, MAX_SUGGESTIONS) : []
	);
	const open = $derived(focused && !dismissed && suggestions.length > 0);

	$effect(() => {
		// A new list invalidates the highlighted option.
		void suggestions;
		active = -1;
	});

	function optionId(index: number) {
		return `${uid}-option-${index}`;
	}

	/** Splits a suggestion into the typed prefix and the completed rest, which is set bold. */
	function split(suggestion: string): [string, string] {
		const typed = query.trimStart().toLowerCase();
		if (typed && suggestion.toLowerCase().startsWith(typed)) {
			return [suggestion.slice(0, typed.length), suggestion.slice(typed.length)];
		}
		return ['', suggestion];
	}

	async function submit(value: string) {
		const q = value.trim();
		if (!q || resolving) return;
		query = q;
		dismissed = true;
		active = -1;

		const link = parseYouTubeUrl(q);
		if (link) {
			resolving = true;
			try {
				const target = await resolveYouTubeUrl(link.href);
				input?.blur();
				await goto(resolve(resolvedPath(target) as '/'));
				return;
			} catch {
				// Not resolvable: search for the text instead.
			} finally {
				resolving = false;
			}
		}
		input?.blur();
		const params = new URLSearchParams({ search_query: q });
		await goto(resolve(`/results?${params.toString()}`));
	}

	function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		void submit(active >= 0 ? suggestions[active] : query);
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.isComposing) return;
		switch (event.key) {
			case 'ArrowDown':
			case 'ArrowUp': {
				if (suggestions.length === 0) return;
				event.preventDefault();
				if (!open) {
					dismissed = false;
					active = event.key === 'ArrowDown' ? 0 : suggestions.length - 1;
					return;
				}
				const step = event.key === 'ArrowDown' ? 1 : -1;
				// -1 (no option) sits between the last and the first option, as in a browser's own list.
				const next = active + step;
				active = next >= suggestions.length ? -1 : next < -1 ? suggestions.length - 1 : next;
				return;
			}
			case 'Escape':
				if (open) {
					event.preventDefault();
					dismissed = true;
					active = -1;
				} else if (query) {
					event.preventDefault();
					query = '';
				}
				return;
			case 'Home':
			case 'End':
				// Keep caret movement in the field; the list has no own focus.
				active = -1;
				return;
		}
	}
</script>

<form role="search" class={cn('relative min-w-0 flex-1 md:max-w-xl', className)} {onsubmit}>
	<div class="relative">
		{#if resolving}
			<LoaderCircleIcon
				class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 animate-spin text-muted-foreground"
				aria-hidden="true"
			/>
		{:else}
			<SearchIcon
				class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
				aria-hidden="true"
			/>
		{/if}
		<Input
			bind:ref={input}
			type="search"
			bind:value={query}
			role="combobox"
			aria-expanded={open}
			aria-controls={listboxId}
			aria-autocomplete="list"
			aria-activedescendant={open && active >= 0 ? optionId(active) : undefined}
			aria-busy={resolving}
			autocomplete="off"
			autocapitalize="off"
			spellcheck="false"
			placeholder={m.search_placeholder()}
			aria-label={m.search_label()}
			aria-keyshortcuts="/"
			enterkeyhint="search"
			data-testid="search-input"
			class="peer bg-muted/60 ps-8 md:pe-9 dark:bg-input/30"
			oninput={() => {
				dismissed = false;
				active = -1;
			}}
			onfocus={() => {
				focused = true;
				dismissed = false;
			}}
			onblur={() => (focused = false)}
			{onkeydown}
		/>
		<kbd
			class="pointer-events-none absolute end-2 top-1/2 hidden h-5 -translate-y-1/2 items-center rounded border bg-background px-1.5 font-sans text-xs text-muted-foreground peer-focus:opacity-0 md:inline-flex"
			title={m.search_shortcut()}
			aria-hidden="true"
		>
			/
		</kbd>
	</div>
	<ul
		id={listboxId}
		role="listbox"
		aria-label={m.search_suggestions_label()}
		data-testid="search-suggestions"
		hidden={!open}
		class="absolute inset-x-0 top-full z-30 mt-1.5 overflow-hidden rounded-lg border bg-popover py-1 text-popover-foreground shadow-lg shadow-black/5 dark:shadow-black/40"
	>
		{#each suggestions as suggestion, index (suggestion)}
			{@const [typed, rest] = split(suggestion)}
			<!-- Options are picked by keyboard through the combobox input, so they take no focus of their own. -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<li
				id={optionId(index)}
				role="option"
				aria-selected={index === active}
				class="flex h-9 cursor-pointer items-center gap-3 px-3 text-sm transition-colors duration-(--motion-out) ease-ui aria-selected:bg-accent aria-selected:text-accent-foreground"
				onpointerdown={(event) => event.preventDefault()}
				onpointermove={() => (active = index)}
				onclick={() => void submit(suggestion)}
			>
				<SearchIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
				<span class="truncate">{typed}<strong class="font-semibold">{rest}</strong></span>
			</li>
		{/each}
	</ul>
	<span class="sr-only" role="status">{resolving ? m.search_resolving() : ''}</span>
</form>
