<script lang="ts">
	import ThumbsUpIcon from '@lucide/svelte/icons/thumbs-up';
	import { LIKED_LIST_ID } from '#lib/api/playlist-order.js';
	import { playlistHref } from '#lib/components/playlist/links.js';
	import PlaylistCover from '#lib/components/playlist/playlist-cover.svelte';
	import VisibilityLabel from '#lib/components/playlist/visibility-label.svelte';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		count,
		thumbnailUrl,
		class: className
	}: {
		/** Unknown while loading. */
		count?: number;
		/** Of the most recently liked video. */
		thumbnailUrl?: string;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<article
	class={cn(
		'group/video relative flex min-w-0 flex-col gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid',
		className
	)}
	data-testid="liked-playlist-card"
>
	<PlaylistCover src={thumbnailUrl} {count} icon={ThumbsUpIcon} />
	<div class="min-w-0">
		<h3 class="line-clamp-2 text-sm leading-5 font-medium text-foreground">
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
			<a
				href={playlistHref(LIKED_LIST_ID)}
				data-card-link
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{m.liked_title()}{#if count !== undefined}<span class="sr-only">
						, {m.playlist_video_count({ count, formatted: formatNumber(count, locale) })}
					</span>{/if}
			</a>
		</h3>
		<p class="mt-1 text-xs text-muted-foreground">
			<VisibilityLabel visibility="private" />
		</p>
	</div>
</article>
