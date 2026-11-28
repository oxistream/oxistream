<script lang="ts">
	import { toVideo } from '#lib/api/mapping.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import type { RoomState } from '#lib/together/connection.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		video: summary,
		onplay,
		class: className
	}: {
		video: NonNullable<RoomState['up_next']>;
		/** Makes it clickable, to play it now. */
		onplay?: () => void;
		class?: string;
	} = $props();

	const uid = $props.id();
	const video = $derived(toVideo(summary));
</script>

<section
	class={cn('flex flex-col', className)}
	aria-labelledby="{uid}-heading"
	data-testid="party-up-next"
>
	<h3
		id="{uid}-heading"
		class="flex h-8 items-center px-2 text-xs font-medium text-sidebar-foreground/70"
	>
		{m.party_up_next()}
	</h3>
	{#snippet content()}
		<VideoThumbnail
			src={video.thumbnailUrl}
			durationSeconds={video.durationSeconds}
			class="w-24 shrink-0 rounded-md"
		/>
		<span class="flex min-w-0 flex-1 flex-col gap-0.5">
			<span class="line-clamp-2 text-sm leading-5 font-medium">{video.title}</span>
			<span class="truncate text-xs text-muted-foreground">{video.channel.name}</span>
			<span class="truncate text-xs text-muted-foreground">{m.party_autoplay()}</span>
		</span>
	{/snippet}
	{#if onplay}
		<button
			type="button"
			class="group/video flex min-w-0 items-start gap-2 rounded-md p-1.5 text-start transition-colors outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-ring"
			title={m.party_play_now_hint()}
			onclick={onplay}
		>
			{@render content()}
		</button>
	{:else}
		<div class="group/video flex min-w-0 items-start gap-2 rounded-md p-1.5">
			{@render content()}
		</div>
	{/if}
</section>
