<script lang="ts">
	import { resolve } from '$app/paths';
	import { toVideo } from '#lib/api/mapping.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomQueueEntry } from '#lib/together/connection.svelte.js';
	import { livePosition, type Playback } from '#lib/together/sync.js';
	import { formatDuration, formatDurationLong } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	const TICK_MS = 250;

	let {
		current,
		playback,
		serverNow = Date.now,
		class: className
	}: {
		current: RoomQueueEntry | null;
		/** The room's playback, for the progress bar. */
		playback?: Playback;
		serverNow?: () => number;
		class?: string;
	} = $props();

	const locale = getLocale();
	const video = $derived(current ? toVideo(current.video) : null);
	let now = $state(0);

	$effect(() => {
		now = serverNow();
		if (!playback?.playing) return;
		const timer = setInterval(() => (now = serverNow()), TICK_MS);
		return () => clearInterval(timer);
	});

	const duration = $derived(video?.durationSeconds ?? 0);
	const position = $derived.by(() => {
		if (!playback) return 0;
		const live = livePosition(playback, now);
		return duration > 0 ? Math.min(live, duration) : live;
	});
	const time = $derived(
		duration > 0
			? `${formatDuration(Math.floor(position))} / ${formatDuration(duration)}`
			: formatDuration(Math.floor(position))
	);
	const percent = $derived(duration > 0 ? (position / duration) * 100 : 0);
</script>

<section
	aria-label={m.party_now_playing()}
	class={cn('flex flex-col gap-2', className)}
	data-testid="party-current"
>
	{#if current && video}
		<a href={resolve(`/watch?v=${video.id}`)} class="group/video block rounded-lg" tabindex="-1">
			<VideoThumbnail
				src={video.thumbnailUrl}
				durationSeconds={playback ? undefined : video.durationSeconds}
			/>
		</a>
		{#if playback}
			<div class="flex items-center gap-2 px-1">
				<div
					class="h-1 flex-1 overflow-hidden rounded-full bg-sidebar-accent"
					role="progressbar"
					aria-label={m.party_progress()}
					aria-valuemin={0}
					aria-valuemax={Math.round(duration)}
					aria-valuenow={Math.round(position)}
					aria-valuetext={formatDurationLong(Math.floor(position), locale)}
					data-testid="party-progress"
				>
					<div class="h-full rounded-full bg-primary" style:width="{percent}%"></div>
				</div>
				<span
					class="shrink-0 text-xs whitespace-nowrap text-muted-foreground tabular-nums"
					data-testid="party-time"
				>
					{time}
				</span>
			</div>
		{/if}
		<div class="flex min-w-0 flex-col gap-0.5 px-1">
			<a
				href={resolve(`/watch?v=${video.id}`)}
				class="line-clamp-2 text-sm leading-5 font-semibold outline-none hover:underline focus-visible:underline"
			>
				{video.title}
			</a>
			<p class="truncate text-xs text-muted-foreground">{video.channel.name}</p>
		</div>
	{:else}
		<div
			class="flex h-20 items-center justify-center rounded-lg bg-sidebar-accent text-sm text-muted-foreground"
		>
			{m.party_nothing()}
		</div>
	{/if}
</section>
