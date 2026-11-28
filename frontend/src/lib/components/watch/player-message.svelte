<script lang="ts">
	import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import RadioIcon from '@lucide/svelte/icons/radio';
	import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
	import VideoOffIcon from '@lucide/svelte/icons/video-off';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { WatchProblem } from './errors';

	let {
		problem,
		videoId,
		poster,
		onretry,
		compact = false,
		class: className
	}: {
		problem: WatchProblem;
		videoId?: string;
		/** Dimmed behind the message, so the slot still reads as this video. */
		poster?: string;
		onretry?: () => void;
		/** Icon and title only, e.g. in the mini player. */
		compact?: boolean;
		class?: string;
	} = $props();

	const Icon = $derived(
		problem.kind === 'live'
			? RadioIcon
			: problem.kind === 'unavailable'
				? VideoOffIcon
				: CircleAlertIcon
	);
</script>

<!-- Player-shaped and always dark, like the player it stands in for. -->
<div
	role={problem.kind === 'error' ? 'alert' : 'status'}
	data-testid="player-message"
	data-kind={problem.kind}
	class={cn(
		'dark relative isolate flex aspect-video w-full items-center justify-center overflow-hidden rounded-xl bg-[oklch(0.13_0.004_286)] text-foreground',
		className
	)}
>
	{#if poster}
		<img
			src={poster}
			alt=""
			class="absolute inset-0 -z-10 size-full scale-105 object-cover opacity-25 blur-md"
		/>
	{/if}
	<div
		class={cn(
			'flex max-w-md flex-col items-center text-center',
			compact ? 'gap-1.5 px-3 py-2' : 'gap-3 px-6 py-8'
		)}
	>
		<div
			class={cn(
				'flex items-center justify-center rounded-full bg-white/10',
				compact ? 'size-8' : 'size-11',
				problem.kind === 'live' ? 'text-primary' : 'text-white/80'
			)}
		>
			<Icon class={compact ? 'size-4' : 'size-5'} />
		</div>
		{#if compact}
			<p class="line-clamp-2 text-xs font-medium text-pretty max-sm:sr-only">{problem.title}</p>
		{:else}
			<div class="flex flex-col gap-1">
				<p class="text-base font-semibold tracking-tight text-pretty">{problem.title}</p>
				{#if problem.description}
					<p class="text-sm text-pretty text-white/70">{problem.description}</p>
				{/if}
			</div>
			{#if (problem.retry && onretry) || (problem.youtube && videoId)}
				<div class="mt-1 flex flex-wrap justify-center gap-2">
					{#if problem.retry && onretry}
						<Button variant="secondary" size="sm" onclick={onretry}>
							<RotateCwIcon />
							{m.action_retry()}
						</Button>
					{/if}
					{#if problem.youtube && videoId}
						<Button
							variant="outline"
							size="sm"
							href="https://www.youtube.com/watch?v={encodeURIComponent(videoId)}"
							target="_blank"
							rel="noopener noreferrer"
						>
							<ExternalLinkIcon />
							{m.watch_open_youtube()}
						</Button>
					{/if}
				</div>
			{/if}
		{/if}
	</div>
</div>
