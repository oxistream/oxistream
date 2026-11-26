<script lang="ts">
	import type { ResolvedPathname } from '$app/types';
	import { CHANNEL_SORTS, type ChannelSort, type VideoTab } from '#lib/channel-tabs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import { channelHref } from './links';

	let {
		base,
		tab,
		current,
		class: className
	}: { base: ResolvedPathname; tab: VideoTab; current: ChannelSort; class?: string } = $props();

	const labels: Record<ChannelSort, () => string> = {
		latest: m.channel_sort_latest,
		popular: m.channel_sort_popular,
		oldest: m.channel_sort_oldest
	};
</script>

<ul class={cn('flex flex-wrap gap-2', className)} aria-label={m.channel_sort_label()}>
	{#each CHANNEL_SORTS as sort (sort)}
		<li>
			<a
				href={channelHref(base, tab, { sort })}
				aria-current={sort === current ? 'true' : undefined}
				data-sveltekit-noscroll
				data-sveltekit-replacestate
				class="inline-flex h-8 items-center rounded-md bg-muted px-3 text-sm font-medium text-foreground transition-colors duration-(--motion-in) ease-ui hover:bg-muted-foreground/15 active:scale-[0.97] aria-[current=true]:bg-foreground aria-[current=true]:text-background"
			>
				{labels[sort]()}
			</a>
		</li>
	{/each}
</ul>
