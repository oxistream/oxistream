<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import PlayIcon from '@lucide/svelte/icons/play';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		poster,
		playing,
		queued = null,
		disabled = false,
		error,
		onplaynow,
		onqueue,
		class: className
	}: {
		/** This video's thumbnail. */
		poster?: string;
		/** Title of the party's video, if one plays. */
		playing?: string;
		/** `added` right after this tab queued it, `already` if it was in the queue before. */
		queued?: 'added' | 'already' | null;
		/** E.g. while not connected or while a request is on its way. */
		disabled?: boolean;
		error?: string | null;
		/** Each shows its button when set. */
		onplaynow?: () => void;
		onqueue?: () => void;
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
		class="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-black/65 p-4 text-center text-white sm:gap-3 sm:p-6"
	>
		<UsersRoundIcon class="size-6 text-white/80 max-sm:hidden" />
		<div class="flex flex-col gap-1">
			<p class="text-sm font-semibold sm:text-base">{m.party_elsewhere_title()}</p>
			<p class="line-clamp-2 max-w-md text-xs text-pretty text-white/75 sm:text-sm">
				{playing ? m.party_elsewhere_playing({ title: playing }) : m.party_elsewhere_empty()}
			</p>
		</div>
		{#if onplaynow || onqueue}
			<div class="mt-1 flex flex-wrap justify-center gap-2">
				{#if onplaynow}
					<Button
						size="sm"
						class="rounded-full px-3.5"
						title={m.party_play_now_hint()}
						{disabled}
						onclick={onplaynow}
						data-testid="party-play-now"
					>
						<PlayIcon class="fill-current" />
						{m.party_play_now()}
					</Button>
				{/if}
				{#if onqueue}
					<Button
						variant="secondary"
						size="sm"
						class={cn('rounded-full px-3.5', queued && 'disabled:opacity-100')}
						disabled={disabled || queued !== null}
						onclick={onqueue}
						data-testid="party-queue-add"
					>
						{#if queued}
							<CheckIcon />
							{queued === 'added' ? m.party_queued() : m.party_already_queued()}
						{:else}
							<ListPlusIcon />
							{m.party_queue_add()}
						{/if}
					</Button>
				{/if}
			</div>
			{#if error}
				<p class="text-xs font-medium text-red-300" role="alert">{error}</p>
			{/if}
		{/if}
	</div>
</div>
