<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		poster,
		playing,
		class: className
	}: {
		/** This video's thumbnail. */
		poster?: string;
		/** Title of the party's video, if one plays. */
		playing?: string;
		class?: string;
	} = $props();
</script>

<!-- Where the player would be, while the party plays another video in the mini player. -->
<div
	class={cn('relative aspect-video w-full overflow-hidden rounded-xl bg-muted', className)}
	data-testid="party-elsewhere"
>
	{#if poster}
		<img src={poster} alt="" class="absolute inset-0 size-full object-cover" />
	{/if}
	<div
		class="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-black/60 p-6 text-center text-white"
	>
		<UsersRoundIcon class="size-6 text-white/80" />
		<p class="text-sm font-semibold sm:text-base">{m.party_elsewhere_title()}</p>
		<p class="line-clamp-2 max-w-md text-xs text-pretty text-white/75 sm:text-sm">
			{playing ? m.party_elsewhere_playing({ title: playing }) : m.party_elsewhere_empty()}
		</p>
	</div>
</div>
