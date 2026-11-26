<script lang="ts">
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		hasMore,
		loading,
		failed = false,
		onload,
		endLabel,
		class: className
	}: {
		hasMore: boolean;
		loading: boolean;
		/** The last page failed; the button then retries and nothing loads on its own. */
		failed?: boolean;
		onload: () => void;
		endLabel?: string;
		class?: string;
	} = $props();

	let sentinel = $state<HTMLElement | null>(null);

	// Re-observing after every load fires again while the sentinel is still on screen, so a
	// short first page keeps filling the viewport.
	$effect(() => {
		if (!sentinel || !hasMore || loading || failed) return;
		if (typeof IntersectionObserver === 'undefined') return;
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries.some((entry) => entry.isIntersecting)) onload();
			},
			{ rootMargin: '0px 0px 800px 0px' }
		);
		observer.observe(sentinel);
		return () => observer.disconnect();
	});
</script>

<div bind:this={sentinel} class={cn('flex min-h-14 items-center justify-center py-4', className)}>
	{#if hasMore}
		{#if loading}
			<p class="flex items-center gap-2 text-sm text-muted-foreground" role="status">
				<LoaderCircleIcon class="size-4 animate-spin" aria-hidden="true" />
				{m.search_loading_more()}
			</p>
		{:else if failed}
			<div class="flex flex-col items-center gap-2 text-center" role="alert">
				<p class="text-sm text-muted-foreground">{m.error_load_body()}</p>
				<Button variant="outline" onclick={onload}>
					<RotateCwIcon />
					{m.action_retry()}
				</Button>
			</div>
		{:else}
			<Button variant="outline" onclick={onload}>{m.search_load_more()}</Button>
		{/if}
	{:else if endLabel}
		<p class="text-xs text-muted-foreground">{endLabel}</p>
	{/if}
</div>
