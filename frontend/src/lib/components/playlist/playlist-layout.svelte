<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		hero,
		children
	}: {
		/** Cover, title and actions; sticky beside the list on wide screens. Omit while loading. */
		hero?: Snippet;
		children?: Snippet;
	} = $props();
</script>

{#if hero}
	<div class="playlist-layout">
		<aside class="playlist-hero flex flex-col gap-4">{@render hero()}</aside>
		<section aria-label={m.playlist_items_label()} class="min-w-0">{@render children?.()}</section>
	</div>
{:else}
	<div class="playlist-layout" role="status" aria-label={m.playlist_loading()}>
		<div class="flex flex-col gap-3">
			<Skeleton class="aspect-video w-full rounded-lg" />
			<Skeleton class="h-6 w-3/4" />
			<Skeleton class="h-4 w-1/2" />
		</div>
		<div class="flex flex-col gap-3">
			{#each { length: 6 }, i (i)}
				<div class="flex items-center gap-3">
					<Skeleton class="aspect-video w-32 shrink-0 rounded-md sm:w-40" />
					<Skeleton class="h-4 flex-1" />
				</div>
			{/each}
		</div>
	</div>
{/if}

<style>
	.playlist-layout {
		display: grid;
		gap: 1.5rem;
		max-width: 96rem;
		margin-inline: auto;
	}

	@media (width >= 64rem) {
		.playlist-layout {
			grid-template-columns: 24rem minmax(0, 1fr);
			gap: 2rem;
			align-items: start;
		}

		.playlist-hero {
			position: sticky;
			top: 5rem;
		}
	}
</style>
