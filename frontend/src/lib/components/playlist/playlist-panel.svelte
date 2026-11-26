<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PlayIcon from '@lucide/svelte/icons/play';
	import { tick } from 'svelte';
	import { neighbors } from '#lib/api/playlist-order.js';
	import type { PlaylistDetails } from '#lib/api/playlists.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { formatDurationLong } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import { playlistHref, playlistWatchHref } from './links';
	import VisibilityLabel from './visibility-label.svelte';

	let {
		playlist,
		currentId,
		status = 'ready',
		class: className
	}: {
		playlist?: PlaylistDetails;
		currentId?: string | null;
		/** Why no playlist is shown: still loading, a YouTube ID, or not found. */
		status?: 'ready' | 'loading' | 'unsupported' | 'unavailable';
		class?: string;
	} = $props();

	const locale = getLocale();
	const position = $derived(playlist ? neighbors(playlist.items, currentId) : { index: -1 });
	let list = $state<HTMLOListElement>();

	// Keeps the current item in view inside the panel, without scrolling the page.
	$effect(() => {
		void currentId;
		void playlist?.items.length;
		void tick().then(() => {
			const current = list?.querySelector<HTMLElement>('[aria-current="true"]');
			if (!list || !current) return;
			list.scrollTop = current.offsetTop - list.offsetTop - list.clientHeight / 3;
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
					{status === 'unsupported'
						? m.playlist_unsupported_title()
						: m.playlist_unavailable_title()}
				</h2>
				<p class="text-xs text-muted-foreground">
					{status === 'unsupported' ? m.playlist_unsupported_body() : m.playlist_unavailable_body()}
				</p>
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
					<span class="truncate">{playlist.owner}</span>
					<span aria-hidden="true">·</span>
					<VisibilityLabel visibility={playlist.visibility} />
					<span aria-hidden="true">·</span>
					<span class="tabular">
						{position.index >= 0
							? m.playlist_position({ index: position.index + 1, total: playlist.items.length })
							: m.playlist_video_count({
									count: playlist.items.length,
									formatted: String(playlist.items.length)
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
		{#if playlist.items.length === 0}
			<p class="p-4 text-sm text-muted-foreground">{m.playlist_empty_body()}</p>
		{:else}
			<ol bind:this={list} class="min-h-0 flex-1 overflow-y-auto py-1" aria-label={playlist.title}>
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
		{/if}
	{/if}
</section>
