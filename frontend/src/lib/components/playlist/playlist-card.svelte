<script lang="ts">
	import type { Playlist } from '#lib/api/playlists.js';
	import { formatNumber, formatRelativeTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import { playlistHref } from './links';
	import PlaylistCover from './playlist-cover.svelte';
	import VisibilityLabel from './visibility-label.svelte';

	let {
		playlist,
		now,
		class: className
	}: {
		playlist: Playlist;
		/** Reference time for the relative date; fixed in stories. */
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
	const updated = $derived(formatRelativeTime(playlist.updatedAt, locale, now));
</script>

<article
	class={cn(
		'group/video relative flex min-w-0 flex-col gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid',
		className
	)}
>
	<PlaylistCover src={playlist.thumbnailUrl} count={playlist.itemCount} />
	<div class="min-w-0">
		<h3 class="line-clamp-2 text-sm leading-5 font-medium text-foreground">
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
			<a
				href={playlistHref(playlist.id)}
				data-card-link
				title={playlist.title}
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{playlist.title}<span class="sr-only">
					, {m.playlist_video_count({
						count: playlist.itemCount,
						formatted: formatNumber(playlist.itemCount, locale)
					})}
				</span>
			</a>
		</h3>
		<p class="mt-1 flex flex-wrap items-center gap-x-1.5 text-xs text-muted-foreground">
			<VisibilityLabel visibility={playlist.visibility} />
			{#if updated}
				<span aria-hidden="true">·</span>
				<span>{m.playlist_updated({ time: updated })}</span>
			{/if}
		</p>
	</div>
</article>
