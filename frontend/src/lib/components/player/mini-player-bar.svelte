<script lang="ts">
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import PauseIcon from '@lucide/svelte/icons/pause';
	import PlayIcon from '@lucide/svelte/icons/play';
	import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
	import VolumeOffIcon from '@lucide/svelte/icons/volume-off';
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		title,
		channel,
		href,
		paused,
		muted = false,
		ontoggle,
		onnext,
		queueCount = 0,
		queueOpen = false,
		onqueuetoggle,
		onclose,
		closeLabel,
		class: className
	}: {
		title: string;
		channel?: string;
		/** The watch page the title leads back to. */
		href: string;
		paused: boolean;
		/** Muted for now, e.g. while a short plays over a watch party. */
		muted?: boolean;
		/** Shows the play/pause button when set. */
		ontoggle?: () => void;
		/** Shows the next button when set. */
		onnext?: () => void;
		/** Videos in the up-next queue; the queue button shows when there are any. */
		queueCount?: number;
		queueOpen?: boolean;
		onqueuetoggle?: () => void;
		onclose: () => void;
		/** Defaults to closing the player. */
		closeLabel?: string;
		class?: string;
	} = $props();
</script>

<div class={cn('flex min-w-0 items-center gap-1 ps-3 pe-1.5', className)}>
	<div class="min-w-0 flex-1 py-2">
		<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by the watch page -->
		<a
			{href}
			class="line-clamp-1 text-sm leading-5 font-medium text-foreground outline-none hover:underline focus-visible:underline"
			data-testid="mini-player-title"
		>
			{title}
		</a>
		<!-- eslint-enable svelte/no-navigation-without-resolve -->
		{#if muted}
			<p
				class="flex items-center gap-1 truncate text-xs leading-4 text-muted-foreground"
				data-testid="mini-player-muted"
			>
				<VolumeOffIcon class="size-3.5 shrink-0" />
				<span class="truncate">{m.mini_player_muted()}</span>
			</p>
		{:else if channel}
			<p class="truncate text-xs leading-4 text-muted-foreground">{channel}</p>
		{/if}
	</div>
	{#if ontoggle}
		<Button
			variant="ghost"
			size="icon-sm"
			aria-label={paused ? m.player_play() : m.player_pause()}
			data-testid="mini-player-toggle"
			onclick={ontoggle}
		>
			{#if paused}
				<PlayIcon class="fill-current" />
			{:else}
				<PauseIcon class="fill-current" />
			{/if}
		</Button>
	{/if}
	{#if onnext}
		<Button
			variant="ghost"
			size="icon-sm"
			aria-label={m.queue_next()}
			title={m.queue_next()}
			data-testid="mini-player-next"
			onclick={onnext}
		>
			<SkipForwardIcon class="fill-current" />
		</Button>
	{/if}
	{#if queueCount > 0 && onqueuetoggle}
		<Button
			variant="ghost"
			size="icon-sm"
			class="relative data-[open]:bg-accent data-[open]:text-foreground"
			aria-label={queueOpen ? m.queue_hide() : m.queue_show()}
			title={queueOpen ? m.queue_hide() : m.queue_show()}
			aria-expanded={queueOpen}
			data-open={queueOpen ? '' : undefined}
			data-testid="mini-player-queue"
			onclick={onqueuetoggle}
		>
			<ListVideoIcon />
			<span
				class="absolute -end-0.5 -top-0.5 grid h-4 min-w-4 place-items-center rounded-full bg-primary px-1 text-[0.625rem] leading-none font-semibold text-primary-foreground tabular-nums"
				aria-hidden="true"
			>
				{queueCount}
			</span>
		</Button>
	{/if}
	<Button
		variant="ghost"
		size="icon-sm"
		aria-label={closeLabel ?? m.mini_player_close()}
		title={closeLabel}
		data-testid="mini-player-close"
		onclick={onclose}
	>
		<XIcon />
	</Button>
</div>
