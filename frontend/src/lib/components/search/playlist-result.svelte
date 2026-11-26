<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import { resolve } from '$app/paths';
	import VerifiedBadge from '#lib/components/channel/verified-badge.svelte';
	import type { PlaylistResult } from '#lib/api/search.js';
	import { formatCompactNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let { playlist, class: className }: { playlist: PlaylistResult; class?: string } = $props();

	const locale = getLocale();
	const href = $derived(
		resolve(`/playlist?${new URLSearchParams({ list: playlist.id }).toString()}`)
	);
</script>

<article
	class={cn(
		'group/video relative grid min-w-0 gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid @xl:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] @xl:gap-4 @4xl:grid-cols-[22rem_minmax(0,1fr)]',
		className
	)}
>
	<!-- A second, offset frame behind the thumbnail marks it as a stack of videos. -->
	<div class="relative pt-1.5" aria-hidden="true">
		<div class="absolute inset-x-3 top-0 h-3 rounded-t-lg bg-muted-foreground/25"></div>
		<div class="relative aspect-video overflow-hidden rounded-lg bg-muted">
			{#if playlist.thumbnailUrl}
				<img
					src={playlist.thumbnailUrl}
					alt=""
					loading="lazy"
					decoding="async"
					width="640"
					height="360"
					class="size-full object-cover transition-transform duration-(--motion-image) ease-ui group-hover/video:scale-[1.03]"
				/>
			{/if}
			<span
				class="absolute end-1.5 bottom-1.5 inline-flex items-center gap-1 rounded-sm bg-overlay px-1.5 py-px text-xs font-medium text-overlay-foreground tabular"
			>
				<ListVideoIcon class="size-3.5" />
				{#if playlist.videoCount !== undefined}
					{formatCompactNumber(playlist.videoCount, locale)}
				{:else}
					{m.search_playlist()}
				{/if}
			</span>
		</div>
	</div>
	<div class="flex min-w-0 flex-col gap-1 @xl:pt-2">
		<h3 class="line-clamp-2 text-sm leading-5 font-medium @xl:text-base @xl:leading-6">
			<a
				{href}
				data-card-link
				title={playlist.title}
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{playlist.title}
			</a>
		</h3>
		<p class="flex min-w-0 items-center gap-1 text-xs text-muted-foreground tabular">
			<span class="shrink-0">{m.search_playlist()}</span>
			{#if playlist.channel}
				<span aria-hidden="true">·</span>
				<span class="truncate">{playlist.channel.name}</span>
				{#if playlist.channel.verified}<VerifiedBadge class="text-sm" />{/if}
			{/if}
			{#if playlist.videoCount !== undefined}
				<span aria-hidden="true">·</span>
				<span class="shrink-0">
					{m.search_playlist_videos({
						count: playlist.videoCount,
						formatted: formatCompactNumber(playlist.videoCount, locale)
					})}
				</span>
			{/if}
		</p>
	</div>
</article>
