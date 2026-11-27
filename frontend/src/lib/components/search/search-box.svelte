<script lang="ts">
	import HistoryIcon from '@lucide/svelte/icons/history';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import SearchIcon from '@lucide/svelte/icons/search';
	import XIcon from '@lucide/svelte/icons/x';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { createSuggestionsQuery, resolveYouTubeUrl } from '#lib/api/search.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { MAX_RECENT_SEARCHES, matchRecent } from '#lib/recent-searches.js';
	import { recentSearches } from '#lib/recent-searches.svelte.js';
	import { cn } from '#lib/utils.js';
	import { parseYouTubeUrl, resolvedPath } from '#lib/youtube.js';

	let {
		input = $bindable(null),
		debounceMs = 180,
		size = 'default',
		class: className
	}: {
		input?: HTMLInputElement | null;
		debounceMs?: number;
		/** `lg` is a standalone box in page content, without the `/` shortcut of the header's. */
		size?: 'default' | 'lg';
		class?: string;
	} = $props();

	const large = $derived(size === 'lg');

	const uid = $props.id();
	const listboxId = `${uid}-suggestions`;
	const MAX_SUGGESTIONS = 8;
	const MAX_RECENT_MATCHES = 3;

	type Option = { kind: 'recent' | 'suggestion' | 'clear'; value: string };

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
	const options = $derived.by((): Option[] => {
		if (isUrl) return [];
		const typed = query.trim();
		if (!typed) {
			const recent = recentSearches.items.slice(0, MAX_RECENT_SEARCHES);
			if (recent.length === 0) return [];
			return [
				...recent.map((value): Option => ({ kind: 'recent', value })),
				{ kind: 'clear', value: '' }
			];
		}
		const recent = matchRecent(recentSearches.items, typed, MAX_RECENT_MATCHES);
		const seen = new Set(recent.map((value) => value.toLowerCase()));
		const suggested = (suggestionsQuery.data ?? []).filter(
			(value) => !seen.has(value.toLowerCase())
		);
		return [
			...recent.map((value): Option => ({ kind: 'recent', value })),
			...suggested.map((value): Option => ({ kind: 'suggestion', value }))
		].slice(0, MAX_SUGGESTIONS);
	});
	const open = $derived(focused && !dismissed && options.length > 0);
	const optionsKey = $derived(options.map((option) => `${option.kind}:${option.value}`).join('\n'));

	/** Highlight to keep after the list changes, e.g. when a recent search is removed. */
	let keepActive: number | null = null;

	$effect(() => {
		// A new list invalidates the highlighted option.
		void optionsKey;
		active = keepActive !== null ? Math.min(keepActive, options.length - 1) : -1;
		keepActive = null;
	});

	function optionId(index: number) {
		return `${uid}-option-${index}`;
	}

	/** Splits a suggestion into the typed prefix and the completed rest, which is set bold. */
	function split(suggestion: string): [string, string] {
		const typed = query.trimStart().toLowerCase();
		if (!typed.trim()) return [suggestion, ''];
		if (suggestion.toLowerCase().startsWith(typed)) {
			return [suggestion.slice(0, typed.length), suggestion.slice(typed.length)];
		}
		return ['', suggestion];
	}

	function removeRecent(value: string, index: number) {
		keepActive = active >= 0 ? index : null;
		recentSearches.remove(value);
	}

	function pick(option: Option) {
		if (option.kind === 'clear') {
			recentSearches.clear();
			return;
		}
		void submit(option.value);
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
		recentSearches.add(q);
		const params = new URLSearchParams({ search_query: q });
		await goto(resolve(`/results?${params.toString()}`));
	}

	function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		if (active >= 0) pick(options[active]);
		else void submit(query);
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.isComposing) return;
		switch (event.key) {
			case 'ArrowDown':
			case 'ArrowUp': {
				if (options.length === 0) return;
				event.preventDefault();
				if (!open) {
					dismissed = false;
					active = event.key === 'ArrowDown' ? 0 : options.length - 1;
					return;
				}
				const step = event.key === 'ArrowDown' ? 1 : -1;
				// -1 (no option) sits between the last and the first option, as in a browser's own list.
				const next = active + step;
				active = next >= options.length ? -1 : next < -1 ? options.length - 1 : next;
				return;
			}
			case 'Delete': {
				const option = open && active >= 0 ? options[active] : undefined;
				if (option?.kind !== 'recent') return;
				event.preventDefault();
				removeRecent(option.value, active);
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

<form
	role="search"
	class={cn('relative min-w-0 flex-1', !large && 'md:max-w-xl', className)}
	{onsubmit}
>
	<div class="relative">
		{#if resolving}
			<LoaderCircleIcon
				class={cn(
					'pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 animate-spin text-muted-foreground',
					large && 'start-4 size-5'
				)}
				aria-hidden="true"
			/>
		{:else}
			<SearchIcon
				class={cn(
					'pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground',
					large && 'start-4 size-5'
				)}
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
			aria-keyshortcuts={large ? undefined : '/'}
			enterkeyhint="search"
			data-testid={large ? 'search-input-large' : 'search-input'}
			class={cn(
				'peer bg-muted/60 ps-8 md:pe-9 dark:bg-input/30',
				large && 'h-12 rounded-xl ps-12 text-base md:pe-4 md:text-base'
			)}
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
		{#if !large}
			<kbd
				class="pointer-events-none absolute end-2 top-1/2 hidden h-5 -translate-y-1/2 items-center rounded border bg-background px-1.5 font-sans text-xs text-muted-foreground peer-focus:opacity-0 md:inline-flex"
				title={m.search_shortcut()}
				aria-hidden="true"
			>
				/
			</kbd>
		{/if}
	</div>
	<ul
		id={listboxId}
		role="listbox"
		aria-label={m.search_suggestions_label()}
		data-testid="search-suggestions"
		hidden={!open}
		class="absolute inset-x-0 top-full z-30 mt-1.5 overflow-hidden rounded-lg border bg-popover py-1 text-popover-foreground shadow-lg shadow-black/5 dark:shadow-black/40"
	>
		{#each options as option, index (`${option.kind}:${option.value}`)}
			<!-- Options are picked by keyboard through the combobox input, so they take no focus of their own. -->
			{#if option.kind === 'clear'}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<li
					id={optionId(index)}
					role="option"
					aria-selected={index === active}
					data-testid="search-recent-clear"
					class="mt-1 flex h-9 cursor-pointer items-center border-t px-3 text-xs text-muted-foreground transition-colors duration-(--motion-out) ease-ui aria-selected:bg-accent aria-selected:text-accent-foreground"
					onpointerdown={(event) => event.preventDefault()}
					onpointermove={() => (active = index)}
					onclick={() => pick(option)}
				>
					{m.search_recent_clear()}
				</li>
			{:else}
				{@const [typed, rest] = split(option.value)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<li
					id={optionId(index)}
					role="option"
					aria-selected={index === active}
					aria-label={option.value}
					data-kind={option.kind}
					class="group/option flex h-9 cursor-pointer items-center gap-3 ps-3 text-sm transition-colors duration-(--motion-out) ease-ui aria-selected:bg-accent aria-selected:text-accent-foreground"
					class:pe-3={option.kind === 'suggestion'}
					class:pe-1={option.kind === 'recent'}
					onpointerdown={(event) => event.preventDefault()}
					onpointermove={() => (active = index)}
					onclick={() => pick(option)}
				>
					{#if option.kind === 'recent'}
						<HistoryIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
					{:else}
						<SearchIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
					{/if}
					<span class="min-w-0 flex-1 truncate">
						{typed}<strong class="font-semibold">{rest}</strong>
					</span>
					{#if option.kind === 'recent'}
						<button
							type="button"
							tabindex="-1"
							aria-label={m.search_recent_remove()}
							title={m.search_recent_remove()}
							class="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors duration-(--motion-out) ease-ui hover:bg-background/70 hover:text-foreground"
							onclick={(event) => {
								event.stopPropagation();
								removeRecent(option.value, index);
							}}
						>
							<XIcon class="size-4" aria-hidden="true" />
						</button>
					{/if}
				</li>
			{/if}
		{/each}
	</ul>
	<span class="sr-only" role="status">{resolving ? m.search_resolving() : ''}</span>
</form>
