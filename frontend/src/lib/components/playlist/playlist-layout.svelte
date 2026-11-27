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

<div class="@container">
	{#if hero}
		<div class="playlist-layout">
			<aside class="playlist-hero flex flex-col gap-4">{@render hero()}</aside>
			<section aria-label={m.playlist_items_label()} class="min-w-0">
				{@render children?.()}
			</section>
		</div>
	{:else}
		<div class="playlist-layout" role="status" aria-label={m.playlist_loading()}>
			<div class="flex flex-col gap-3">
				<Skeleton class="aspect-video w-full rounded-xl" />
				<Skeleton class="mt-1 h-6 w-3/4" />
				<Skeleton class="h-4 w-1/2" />
				<Skeleton class="mt-1 h-9 w-32 rounded-md" />
			</div>
			<div class="flex flex-col gap-1">
				{#each { length: 6 }, i (i)}
					<div class="flex items-start gap-3 py-2">
						<Skeleton class="aspect-video w-36 shrink-0 rounded-lg sm:w-44" />
						<div class="flex flex-1 flex-col gap-2 pt-0.5">
							<Skeleton class="h-4 w-11/12" />
							<Skeleton class="h-3 w-1/3" />
							<Skeleton class="h-3 w-1/4" />
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.playlist-layout {
		display: grid;
		grid-template-columns: minmax(0, 44rem);
		justify-content: center;
		gap: 1.5rem;
	}

	@container (width >= 50rem) {
		.playlist-layout {
			grid-template-columns: minmax(15rem, 20rem) minmax(0, 44rem);
			gap: 2.5rem;
			align-items: start;
		}

		.playlist-hero {
			position: sticky;
			top: 5rem;
			max-height: calc(100dvh - 6rem);
			overflow-y: auto;
			scrollbar-width: thin;
			/* Room for focus rings inside the scroll box. */
			margin: -0.25rem;
			padding: 0.25rem;
		}
	}
</style>
