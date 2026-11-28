<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PlayIcon from '@lucide/svelte/icons/play';
	import { tick } from 'svelte';
	import { neighbors } from '#lib/api/playlist-order.js';
	import InfiniteLoader from '#lib/components/state/infinite-loader.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { formatDurationLong } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import { playlistHref, playlistWatchHref } from './links';
	import type { PlaybackList } from './playback';
	import VisibilityLabel from './visibility-label.svelte';

	let {
		playlist,
		currentId,
		status = 'ready',
		hasMore = false,
		loadingMore = false,
		loadFailed = false,
		onloadmore,
		class: className
	}: {
		playlist?: PlaybackList;
		currentId?: string | null;
		/** Why no playlist is shown: still loading, a YouTube mix, or not found. */
		status?: 'ready' | 'loading' | 'mix' | 'unavailable';
		/** More items can be loaded, e.g. further pages of a YouTube playlist. */
		hasMore?: boolean;
		loadingMore?: boolean;
		loadFailed?: boolean;
		onloadmore?: () => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	const position = $derived(playlist ? neighbors(playlist.items, currentId) : { index: -1 });
	let scroller = $state<HTMLDivElement>();
	let scrolledTo: string | null | undefined;

	// Keeps the current item in view inside the panel, without scrolling the page. Loading more
	// items leaves the scroll position alone, so it does not fight the user scrolling the list.
	$effect(() => {
		const id = currentId;
		void playlist?.items.length;
		void tick().then(() => {
			const current = scroller?.querySelector<HTMLElement>('[aria-current="true"]');
			if (!scroller || !current || scrolledTo === id) return;
			scrolledTo = id;
			scroller.scrollTop = current.offsetTop - scroller.offsetTop - scroller.clientHeight / 3;
		});
	});
</script>

<section
	aria-labelledby="playlist-panel-heading"
	class={cn('flex min-h-0 flex-col overflow-hidden rounded-xl border bg-card', className)}
	data-testid="playlist-panel"
	data-status={status}
>
	{#if status === 'loading'}
		<div class="flex flex-col gap-3 p-4" role="status" aria-label={m.playlist_loading()}>
			<h2 id="playlist-panel-heading" class="sr-only">{m.playlist_loading()}</h2>
			<Skeleton class="h-4 w-2/3" />
			<Skeleton class="h-3 w-1/3" />
			{#each { length: 3 }, i (i)}
				<div class="flex gap-3">
					<Skeleton class="aspect-video w-24 shrink-0 rounded-md" />
					<Skeleton class="h-3.5 flex-1" />
				</div>
			{/each}
		</div>
	{:else if status !== 'ready' || !playlist}
		<div class="flex items-start gap-3 p-4">
			<ListVideoIcon class="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
			<div class="flex flex-col gap-0.5">
				<h2 id="playlist-panel-heading" class="text-sm font-semibold">
					{status === 'mix' ? m.playlist_mix_title() : m.playlist_unavailable_title()}
				</h2>
				{#if status !== 'mix'}
					<p class="text-xs text-muted-foreground">{m.playlist_unavailable_body()}</p>
				{/if}
			</div>
		</div>
	{:else}
		<header class="flex items-start justify-between gap-3 border-b px-4 py-3">
			<div class="min-w-0">
				<h2 id="playlist-panel-heading" class="truncate text-sm font-semibold">
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
					<a href={playlistHref(playlist.id)} class="rounded-sm hover:text-primary">
						{playlist.title}
					</a>
				</h2>
				<p class="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-xs text-muted-foreground">
					{#if playlist.owner}
						<span class="truncate">{playlist.owner}</span>
						<span aria-hidden="true">·</span>
					{/if}
					{#if playlist.visibility}
						<VisibilityLabel visibility={playlist.visibility} />
						<span aria-hidden="true">·</span>
					{/if}
					<span class="tabular">
						{position.index >= 0
							? m.playlist_position({ index: position.index + 1, total: playlist.total })
							: m.playlist_video_count({
									count: playlist.total,
									formatted: String(playlist.total)
								})}
					</span>
				</p>
			</div>
			<div class="flex shrink-0 gap-1">
				{#if position.previous}
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
					<Button
						href={playlistWatchHref(position.previous.id, playlist.id)}
						variant="ghost"
						size="icon-sm"
						aria-label={m.playlist_previous()}
						title={m.playlist_previous()}
					>
						<ChevronLeftIcon />
					</Button>
				{:else}
					<Button variant="ghost" size="icon-sm" disabled aria-label={m.playlist_previous()}>
						<ChevronLeftIcon />
					</Button>
				{/if}
				{#if position.next}
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
					<Button
						href={playlistWatchHref(position.next.id, playlist.id)}
						variant="ghost"
						size="icon-sm"
						aria-label={m.playlist_next()}
						title={m.playlist_next()}
						data-testid="playlist-next"
					>
						<ChevronRightIcon />
					</Button>
				{:else}
					<Button variant="ghost" size="icon-sm" disabled aria-label={m.playlist_next()}>
						<ChevronRightIcon />
					</Button>
				{/if}
			</div>
		</header>
		{#if playlist.items.length === 0 && !hasMore}
			<p class="p-4 text-sm text-muted-foreground">{m.playlist_empty_title()}</p>
		{:else}
			<div bind:this={scroller} class="min-h-0 flex-1 overflow-y-auto">
				<ol class="py-1" aria-label={playlist.title}>
					{#each playlist.items as video, i (video.id)}
						{@const current = i === position.index}
						<li>
							<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved in links.ts -->
							<a
								href={playlistWatchHref(video.id, playlist.id)}
								aria-current={current ? 'true' : undefined}
								class={cn(
									'group/video flex items-center gap-2.5 px-2 py-1.5 transition-colors duration-(--motion-in) ease-ui hover:bg-muted focus-visible:-outline-offset-2',
									current && 'bg-muted'
								)}
							>
								<span
									class="flex w-5 shrink-0 justify-center text-xs text-muted-foreground tabular"
									aria-hidden="true"
								>
									{#if current}
										<PlayIcon class="size-3 fill-primary text-primary" />
									{:else}
										{i + 1}
									{/if}
								</span>
								<VideoThumbnail
									src={video.thumbnailUrl}
									durationSeconds={video.durationSeconds}
									class="w-24 shrink-0 rounded-md"
								/>
								<span class="min-w-0 flex-1">
									<span class="line-clamp-2 text-[0.8125rem] leading-[1.125rem] font-medium">
										{video.title}
									</span>
									<span class="mt-0.5 block truncate text-xs text-muted-foreground">
										{video.channel.name}
									</span>
									{#if video.durationSeconds !== undefined}
										<span class="sr-only">
											, {m.video_duration({
												duration: formatDurationLong(video.durationSeconds, locale)
											})}
										</span>
									{/if}
								</span>
							</a>
						</li>
					{/each}
				</ol>
				{#if hasMore && onloadmore}
					<InfiniteLoader
						hasMore
						loading={loadingMore}
						failed={loadFailed}
						onload={onloadmore}
						class="min-h-12 py-2"
					/>
				{/if}
			</div>
		{/if}
	{/if}
</section>
