<script lang="ts">
	import type { FollowedPlaylist } from '#lib/api/playlist-follows.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import { playlistHref } from './links';
	import PlaylistCover from './playlist-cover.svelte';

	let { playlist, class: className }: { playlist: FollowedPlaylist; class?: string } = $props();

	const locale = getLocale();
	const pending = $derived(playlist.title === undefined && !playlist.unavailable);
	const newCount = $derived(
		playlist.newVideoCount > 0
			? m.playlist_new_videos({
					count: playlist.newVideoCount,
					formatted: formatNumber(playlist.newVideoCount, locale)
				})
			: undefined
	);
</script>

<article
	class={cn(
		'group/video relative flex min-w-0 flex-col gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid',
		className
	)}
	data-testid="followed-playlist"
>
	<div class="relative">
		<PlaylistCover src={playlist.thumbnailUrl} count={playlist.videoCount} />
		{#if newCount}
			<span
				class="absolute start-1.5 bottom-1.5 rounded-sm bg-primary px-1.5 py-px text-xs font-medium text-primary-foreground tabular"
				aria-hidden="true"
			>
				{newCount}
			</span>
		{/if}
	</div>
	<div class="min-w-0">
		<h3 class="line-clamp-2 text-sm leading-5 font-medium text-foreground">
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
			<a
				href={playlistHref(playlist.id)}
				data-card-link
				title={playlist.title}
				class={cn(
					'outline-none after:absolute after:inset-0 after:rounded-lg after:content-[""]',
					!playlist.title && 'text-muted-foreground'
				)}
			>
				{playlist.title ?? (pending ? m.playlist_pending() : playlist.id)}{#if newCount}<span
						class="sr-only"
					>
						, {newCount}
					</span>{/if}
			</a>
		</h3>
		<p class="mt-1 flex flex-wrap items-center gap-x-1.5 text-xs text-muted-foreground">
			{#if playlist.unavailable}
				<span class="text-destructive">{m.playlist_follow_unavailable()}</span>
			{:else}
				{#if playlist.channel}
					<span class="truncate">{playlist.channel.name}</span>
					<span aria-hidden="true">·</span>
				{/if}
				<span>{m.playlist_source_youtube()}</span>
			{/if}
		</p>
	</div>
</article>
