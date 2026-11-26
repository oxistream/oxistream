<script lang="ts">
	import type { PlaylistResult } from '#lib/api/search.js';
	import CourseBadge from '#lib/components/playlist/course-badge.svelte';
	import { playlistHref } from '#lib/components/playlist/links.js';
	import PlaylistCover from '#lib/components/playlist/playlist-cover.svelte';
	import VideoCardSkeleton from '#lib/components/video/video-card-skeleton.svelte';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		playlists = [],
		loading = false,
		skeletonCount = 8,
		label,
		class: className
	}: {
		playlists?: PlaylistResult[];
		loading?: boolean;
		skeletonCount?: number;
		label?: string;
		class?: string;
	} = $props();

	const locale = getLocale();
</script>

<div class={cn('@container', className)}>
	{#if loading}
		<div class="video-list" data-layout="grid" role="status" aria-label={m.video_loading()}>
			{#each { length: skeletonCount }, i (i)}
				<VideoCardSkeleton showChannel={false} />
			{/each}
		</div>
	{:else}
		<ul class="video-list" data-layout="grid" aria-label={label}>
			{#each playlists as playlist (playlist.id)}
				<li class="min-w-0">
					<article
						class="group/video relative flex min-w-0 flex-col gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid"
					>
						<PlaylistCover src={playlist.thumbnailUrl} count={playlist.videoCount} />
						<h3 class="line-clamp-2 text-sm leading-5 font-medium text-foreground">
							<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
							<a
								href={playlistHref(playlist.id)}
								data-card-link
								title={playlist.title}
								class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
							>
								{playlist.title}{#if playlist.videoCount !== undefined}<span class="sr-only">
										, {m.playlist_video_count({
											count: playlist.videoCount,
											formatted: formatNumber(playlist.videoCount, locale)
										})}
									</span>{/if}
							</a>
						</h3>
						{#if playlist.isCourse}<CourseBadge class="-mt-1.5" />{/if}
					</article>
				</li>
			{/each}
		</ul>
	{/if}
</div>
