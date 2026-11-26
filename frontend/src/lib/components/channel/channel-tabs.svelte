<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import XIcon from '@lucide/svelte/icons/x';
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import type { ResolvedPathname } from '$app/types';
	import { visibleTabs, type ChannelTab } from '#lib/channel-tabs.js';
	import { tabLinkClass } from '#lib/components/tab-link.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import { channelHref } from './links';

	let {
		base,
		current,
		hasShorts,
		hasLive,
		query = '',
		class: className
	}: {
		/** Channel path without tab, `/@handle` or `/channel/UC…`. */
		base: ResolvedPathname;
		current: ChannelTab;
		hasShorts: boolean;
		hasLive: boolean;
		/** Query of the search tab, shown in the open search field. */
		query?: string;
		class?: string;
	} = $props();

	const labels: Record<Exclude<ChannelTab, 'search'>, () => string> = {
		videos: m.channel_videos,
		shorts: m.channel_tab_shorts,
		streams: m.channel_tab_live,
		playlists: m.channel_tab_playlists
	};

	const tabs = $derived(visibleTabs({ hasShorts, hasLive }, current));
	let open = $state(false);
	// Follows the search tab's query and can be typed over.
	let value = $derived(query);
	let input = $state<HTMLInputElement | null>(null);
	const expanded = $derived(open || current === 'search');

	async function expand() {
		open = true;
		await tick();
		input?.focus();
	}

	function collapse() {
		value = '';
		open = false;
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const q = value.trim();
		if (!q) return;
		void goto(channelHref(base, 'search', { query: q }));
	}
</script>

<div class={cn('flex min-w-0 items-end gap-2 border-b', className)}>
	<nav aria-label={m.channel_tabs_label()} class="-mb-px min-w-0 flex-1 overflow-x-auto">
		<ul class="flex gap-1">
			{#each tabs as tab (tab)}
				<li class="shrink-0">
					<a
						href={channelHref(base, tab)}
						aria-current={tab === current ? 'page' : undefined}
						data-sveltekit-noscroll
						class={tabLinkClass}
					>
						{labels[tab]()}
					</a>
				</li>
			{/each}
		</ul>
	</nav>
	<div class="flex shrink-0 items-center pb-1">
		{#if expanded}
			<form role="search" class="relative flex items-center" onsubmit={submit}>
				<SearchIcon
					class="pointer-events-none absolute start-2.5 size-4 text-muted-foreground"
					aria-hidden="true"
				/>
				<input
					bind:this={input}
					bind:value
					type="search"
					name="query"
					enterkeyhint="search"
					autocomplete="off"
					aria-label={m.channel_search_label()}
					placeholder={m.channel_search_placeholder()}
					data-testid="channel-search-input"
					onkeydown={(event) => {
						if (event.key === 'Escape' && current !== 'search') collapse();
					}}
					onblur={() => {
						if (!value.trim() && current !== 'search') collapse();
					}}
					class="h-8 w-44 rounded-md border bg-background ps-8 pe-8 text-sm outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40 sm:w-60 [&::-webkit-search-cancel-button]:hidden"
				/>
				{#if value}
					<button
						type="button"
						class="absolute end-1 inline-flex size-6 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:text-foreground"
						aria-label={m.channel_search_clear()}
						onclick={() => {
							value = '';
							input?.focus();
						}}
					>
						<XIcon class="size-3.5" aria-hidden="true" />
					</button>
				{/if}
			</form>
		{:else}
			<button
				type="button"
				class="inline-flex size-8 items-center justify-center rounded-md text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:bg-muted hover:text-foreground active:scale-95"
				aria-label={m.channel_search_open()}
				data-testid="channel-search-toggle"
				onclick={expand}
			>
				<SearchIcon class="size-4" aria-hidden="true" />
			</button>
		{/if}
	</div>
</div>
