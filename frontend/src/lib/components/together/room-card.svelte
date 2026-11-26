<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { resolve } from '$app/paths';
	import type { RoomListItem } from '#lib/api/rooms.js';
	import VideoThumbnail from '#lib/components/video/video-thumbnail.svelte';
	import { formatNumber, formatRelativeTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	let {
		room,
		now,
		class: className
	}: {
		room: RoomListItem;
		/** Reference time for the relative date; fixed in stories. */
		now?: Date;
		class?: string;
	} = $props();

	const locale = getLocale();
	const created = $derived(formatRelativeTime(room.createdAt, locale, now));
	const title = $derived(room.currentTitle ?? m.together_room_nothing());
</script>

<article
	class={cn(
		'group/video relative flex min-w-0 flex-col gap-3 rounded-lg outline-offset-4 outline-ring has-[[data-card-link]:focus-visible]:outline-2 has-[[data-card-link]:focus-visible]:outline-solid',
		className
	)}
>
	<div class="relative">
		<VideoThumbnail src={room.thumbnailUrl} />
		<span
			class="absolute start-2 top-2 inline-flex items-center gap-1 rounded-md bg-black/70 px-1.5 py-0.5 text-xs font-medium text-white tabular-nums"
			aria-hidden="true"
		>
			<UsersRoundIcon class="size-3.5" />
			{formatNumber(room.participantCount, locale)}
		</span>
	</div>
	<div class="min-w-0">
		<h3 class="line-clamp-2 text-sm leading-5 font-medium text-foreground">
			<a
				href={resolve('/together/[room]', { room: room.id })}
				data-card-link
				{title}
				class="outline-none after:absolute after:inset-0 after:rounded-lg after:content-['']"
			>
				{title}
			</a>
		</h3>
		<p class="mt-1 flex flex-wrap items-center gap-x-1.5 text-xs text-muted-foreground">
			<span>
				{m.together_room_meta({
					people: formatNumber(room.participantCount, locale),
					queue: formatNumber(room.queueCount, locale)
				})}
			</span>
			<span aria-hidden="true">·</span>
			<span>{created}</span>
		</p>
	</div>
</article>
