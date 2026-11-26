<script lang="ts">
	import { resolve } from '$app/paths';
	import { tabLinkClass } from '#lib/components/tab-link.js';
	import { m } from '#lib/paraglide/messages.js';
	import { SEARCH_TYPES, searchParams, type SearchType } from '#lib/search-types.js';
	import { cn } from '#lib/utils.js';

	let {
		query,
		current,
		class: className
	}: {
		query: string;
		current: SearchType;
		class?: string;
	} = $props();

	const labels: Record<SearchType, () => string> = {
		all: m.search_tab_all,
		videos: m.search_tab_videos,
		shorts: m.search_tab_shorts,
		channels: m.search_tab_channels,
		playlists: m.search_tab_playlists
	};
</script>

<div class={cn('min-w-0 border-b', className)}>
	<nav aria-label={m.search_tabs_label()} class="-mb-px overflow-x-auto">
		<ul class="flex gap-1">
			{#each SEARCH_TYPES as type (type)}
				<li class="shrink-0">
					<a
						href={resolve(`/results?${searchParams(query, type)}`)}
						aria-current={type === current ? 'page' : undefined}
						data-sveltekit-noscroll
						data-testid="search-tab-{type}"
						class={tabLinkClass}
					>
						{labels[type]()}
					</a>
				</li>
			{/each}
		</ul>
	</nav>
</div>
