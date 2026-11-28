<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import LocateFixedIcon from '@lucide/svelte/icons/locate-fixed';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { tick } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import type { TranscriptLine } from './transcript';
	import { useTranscript } from '#lib/api/transcript.js';
	import type { Subtitle } from '#lib/api/video.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { formatDuration } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import { indexAt } from './chapters';
	import { defaultTrack, findMatches, highlight } from './transcript';

	let {
		videoId,
		tracks,
		time = null,
		showing = null,
		audioLang = null,
		onseek,
		class: className
	}: {
		videoId: string;
		tracks: Subtitle[];
		/** Where this video plays, `null` while it is not loaded in the player. */
		time?: number | null;
		/** The `id` of the subtitles showing in the player. */
		showing?: string | null;
		/** Language of the video's original audio track. */
		audioLang?: string | null;
		/** Without it, lines are not clickable, e.g. for party members who may not seek. */
		onseek?: (seconds: number) => void;
		class?: string;
	} = $props();

	const label = (track: Subtitle) =>
		track.auto_generated ? m.player_auto_generated({ label: track.label }) : track.label;

	let picked = $state<string | null>(null);
	// Turning on other captions in the player switches the transcript to them.
	$effect(() => {
		void showing;
		picked = null;
	});
	const track = $derived(
		tracks.find((t) => t.id === picked) ?? defaultTrack(tracks, { showing, audioLang })
	);
	const transcript = useTranscript(() => track && { videoId, id: track.id, url: track.url });
	const lines = $derived(transcript.data ?? []);

	let query = $state('');
	const matches = $derived(findMatches(lines, query));
	let matchIndex = $state(0);
	$effect(() => {
		void matches;
		matchIndex = 0;
	});
	const matched = $derived(new Set(matches));
	const activeMatch = $derived(matches.length ? matches[matchIndex % matches.length] : -1);

	const current = $derived(time === null ? -1 : indexAt(lines, time));
	let following = $state(true);
	let scroller = $state<HTMLElement>();
	/** Until then, scroll events are our own. */
	let ownScrollUntil = 0;

	function scrollToLine(index: number, behavior: ScrollBehavior = 'smooth') {
		const row = scroller?.querySelector<HTMLElement>(`[data-index="${index}"]`);
		if (!scroller || !row) return;
		ownScrollUntil = performance.now() + 800;
		const top = row.offsetTop - scroller.clientHeight / 2 + row.offsetHeight / 2;
		scroller.scrollTo({ top: Math.max(0, top), behavior });
	}

	$effect(() => {
		const index = current;
		if (!following || index < 0) return;
		void tick().then(() => scrollToLine(index));
	});

	// Wheel, touch, keys and the scrollbar mean the user reads elsewhere; following waits for
	// "resume".
	const SCROLL_KEYS = new Set(['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' ']);
	const watchUserScroll: Attachment<HTMLElement> = (node) => {
		const stop = () => {
			if (performance.now() >= ownScrollUntil) following = false;
		};
		const onkey = (event: KeyboardEvent) => {
			if (SCROLL_KEYS.has(event.key)) stop();
		};
		// A press on the scroller itself, not on a line, is on its scrollbar.
		const onpointer = (event: PointerEvent) => {
			if (event.target === node) stop();
		};
		node.addEventListener('wheel', stop, { passive: true });
		node.addEventListener('touchmove', stop, { passive: true });
		node.addEventListener('keydown', onkey);
		node.addEventListener('pointerdown', onpointer);
		return () => {
			node.removeEventListener('wheel', stop);
			node.removeEventListener('touchmove', stop);
			node.removeEventListener('keydown', onkey);
			node.removeEventListener('pointerdown', onpointer);
		};
	};

	function resume() {
		following = true;
		if (current >= 0) scrollToLine(current);
	}

	function step(delta: number) {
		if (!matches.length) return;
		matchIndex = (matchIndex + delta + matches.length) % matches.length;
		following = false;
		void tick().then(() => scrollToLine(matches[matchIndex]));
	}

	function onsearchkey(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			step(event.shiftKey ? -1 : 1);
		} else if (event.key === 'Escape' && query) {
			event.preventDefault();
			query = '';
		}
	}

	$effect(() => {
		if (!query.trim() || !matches.length) return;
		following = false;
		const first = matches[0];
		void tick().then(() => scrollToLine(first, 'auto'));
	});
</script>

<div class={cn('flex min-h-0 flex-col', className)} data-testid="transcript">
	<div class="flex flex-wrap items-center gap-1.5 border-b p-1.5">
		{#if tracks.length > 1 && track}
			<Select.Root
				type="single"
				value={track.id}
				onValueChange={(next) => {
					if (next) picked = next;
				}}
			>
				<Select.Trigger
					size="sm"
					class="max-w-full min-w-0 flex-1 basis-full"
					aria-label={m.transcript_language()}
					data-testid="transcript-language"
				>
					<span class="truncate">{label(track)}</span>
				</Select.Trigger>
				<Select.Content>
					{#each tracks as option (option.id)}
						<Select.Item value={option.id} label={label(option)} />
					{/each}
				</Select.Content>
			</Select.Root>
		{/if}
		<div class="relative min-w-0 flex-1">
			<SearchIcon
				class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
				aria-hidden="true"
			/>
			<input
				type="search"
				bind:value={query}
				onkeydown={onsearchkey}
				placeholder={m.transcript_search()}
				aria-label={m.transcript_search()}
				class="h-8 w-full min-w-0 rounded-md border border-input bg-transparent ps-8 pe-2 text-sm shadow-xs outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/25 dark:bg-input/30"
				data-testid="transcript-search"
			/>
		</div>
		{#if query.trim()}
			<span
				class="min-w-10 text-center text-xs whitespace-nowrap text-muted-foreground tabular"
				aria-live="polite"
				data-testid="transcript-matches"
			>
				{matches.length ? `${matchIndex + 1}/${matches.length}` : '0/0'}
			</span>
			<Button
				variant="ghost"
				size="icon-sm"
				aria-label={m.transcript_previous_match()}
				title={m.transcript_previous_match()}
				disabled={!matches.length}
				onclick={() => step(-1)}
			>
				<ChevronUpIcon />
			</Button>
			<Button
				variant="ghost"
				size="icon-sm"
				aria-label={m.transcript_next_match()}
				title={m.transcript_next_match()}
				disabled={!matches.length}
				onclick={() => step(1)}
				data-testid="transcript-next"
			>
				<ChevronDownIcon />
			</Button>
		{/if}
	</div>

	<div class="relative flex min-h-0 flex-1 flex-col">
		{#if transcript.isPending && track}
			<div class="flex flex-col gap-3 p-3" role="status" aria-label={m.transcript_loading()}>
				{#each { length: 6 }, i (i)}
					<div class="flex gap-3" aria-hidden="true">
						<Skeleton class="h-3.5 w-9" />
						<Skeleton class="h-3.5 flex-1" />
					</div>
				{/each}
			</div>
		{:else if transcript.isError}
			<div class="flex items-center justify-between gap-3 p-3 text-sm text-muted-foreground">
				<span>{m.transcript_error()}</span>
				<Button variant="outline" size="sm" onclick={() => void transcript.refetch()}>
					{m.action_retry()}
				</Button>
			</div>
		{:else if lines.length === 0}
			<p class="p-3 text-sm text-muted-foreground">{m.transcript_empty()}</p>
		{:else}
			<ol
				bind:this={scroller}
				{@attach watchUserScroll}
				class={cn(
					'transcript-lines relative min-h-0 flex-1 overflow-y-auto pt-1.5',
					// Room for the resume button below the last line.
					following ? 'pb-1.5' : 'pb-14'
				)}
				data-testid="transcript-lines"
			>
				{#each lines as line, i (i)}
					<li data-index={i}>
						{#if onseek}
							<button
								type="button"
								class="transcript-line"
								data-seekable=""
								aria-current={i === current ? 'true' : undefined}
								data-match={i === activeMatch ? 'active' : undefined}
								onclick={() => {
									following = true;
									onseek(line.start);
								}}
								data-testid="transcript-line"
							>
								{@render row(line, i)}
							</button>
						{:else}
							<div
								class="transcript-line"
								aria-current={i === current ? 'true' : undefined}
								data-match={i === activeMatch ? 'active' : undefined}
								data-testid="transcript-line"
							>
								{@render row(line, i)}
							</div>
						{/if}
					</li>
				{/each}
			</ol>
			{#if !following && current >= 0}
				<Button
					variant="secondary"
					size="sm"
					class="absolute bottom-3 left-1/2 -translate-x-1/2 rounded-full whitespace-nowrap shadow-md"
					onclick={resume}
					data-testid="transcript-resume"
				>
					<LocateFixedIcon />
					{m.transcript_resume()}
				</Button>
			{/if}
		{/if}
	</div>
</div>

{#snippet row(line: TranscriptLine, i: number)}
	<span class="transcript-time tabular">{formatDuration(line.start)}</span>
	<span class="break-words">
		{#if matched.has(i)}
			{#each highlight(line.text, query) as part, p (p)}
				{#if part.match}<mark>{part.text}</mark>{:else}{part.text}{/if}
			{/each}
		{:else}
			{line.text}
		{/if}
	</span>
{/snippet}

<style>
	.transcript-line {
		display: grid;
		grid-template-columns: 3.25rem minmax(0, 1fr);
		gap: 0.5rem;
		width: 100%;
		padding: 0.25rem 0.75rem;
		color: color-mix(in oklch, var(--foreground) 80%, transparent);
		font-size: 0.875rem;
		line-height: 1.375;
		text-align: left;
	}

	.transcript-line[data-seekable] {
		cursor: pointer;
	}

	.transcript-line[data-seekable]:hover {
		background: color-mix(in oklch, var(--accent) 70%, transparent);
	}

	.transcript-line:focus-visible {
		outline: 2px solid var(--ring);
		outline-offset: -2px;
	}

	.transcript-line[aria-current='true'] {
		background: var(--accent);
		color: var(--foreground);
	}

	.transcript-time {
		padding-top: 1px;
		color: var(--muted-foreground);
		font-size: 0.75rem;
	}

	[aria-current='true'] > .transcript-time {
		color: var(--primary);
		font-weight: 500;
	}

	.transcript-line :global(mark) {
		border-radius: 0.125rem;
		background: color-mix(in oklch, var(--primary) 28%, transparent);
		color: inherit;
	}

	.transcript-line[data-match='active'] :global(mark) {
		background: var(--primary);
		color: var(--primary-foreground);
	}
</style>
