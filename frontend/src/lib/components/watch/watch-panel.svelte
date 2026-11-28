<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import type { VideoDetails } from '#lib/api/video.js';
	import { readJson, writeJson } from '#lib/components/player/storage.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import ChapterList from './chapter-list.svelte';
	import { indexAt, toChapters } from './chapters';
	import TranscriptPanel from './transcript-panel.svelte';

	type Tab = 'chapters' | 'transcript';
	type Stored = { open: boolean; tab: Tab };

	const KEY = 'oxistream:watch-panel';
	const isStored = (v: unknown): v is Stored =>
		typeof v === 'object' &&
		v !== null &&
		typeof (v as Stored).open === 'boolean' &&
		((v as Stored).tab === 'chapters' || (v as Stored).tab === 'transcript');

	let {
		video,
		time = null,
		showing = null,
		onseek,
		class: className
	}: {
		video: VideoDetails;
		/** Where this video plays, `null` while it is not loaded in the player. */
		time?: number | null;
		/** The `id` of the subtitles showing in the player. */
		showing?: string | null;
		onseek?: (seconds: number) => void;
		class?: string;
	} = $props();

	const chapters = $derived(toChapters(video.chapters, video.duration_seconds));
	const tracks = $derived(video.subtitles);
	const audioLang = $derived(video.audio_tracks?.find((t) => t.is_default)?.lang ?? null);
	const currentChapter = $derived(time === null ? -1 : indexAt(chapters, time));

	const stored = readJson(KEY, isStored);
	// Phones start collapsed, below the description.
	let open = $state(stored?.open ?? window.matchMedia('(width >= 64rem)').matches);
	let tab = $state<Tab>(stored?.tab ?? 'chapters');
	const available = $derived<Tab[]>([
		...(chapters.length ? (['chapters'] as const) : []),
		...(tracks.length ? (['transcript'] as const) : [])
	]);
	const shown = $derived(available.includes(tab) ? tab : available[0]);

	function save() {
		writeJson(KEY, { open, tab: shown });
	}

	function select(next: string) {
		if (next !== 'chapters' && next !== 'transcript') return;
		tab = next;
		open = true;
		save();
	}

	function toggle() {
		open = !open;
		save();
	}

	const uid = $props.id();
</script>

{#if shown}
	<section
		class={cn('flex min-h-0 flex-col overflow-hidden rounded-xl border bg-card', className)}
		aria-label={shown === 'chapters' ? m.watch_chapters() : m.watch_transcript()}
		data-testid="watch-panel"
		data-open={open ? '' : undefined}
	>
		<Tabs.Root value={shown} onValueChange={select} class="flex min-h-0 flex-1 flex-col gap-0">
			<header class="flex items-center gap-2 p-1.5">
				<Tabs.List class="h-8">
					{#if chapters.length}
						<Tabs.Trigger value="chapters" class="px-2.5" onclick={() => select('chapters')}>
							{m.watch_chapters()}
							<span class="font-normal text-muted-foreground tabular">{chapters.length}</span>
						</Tabs.Trigger>
					{/if}
					{#if tracks.length}
						<Tabs.Trigger value="transcript" class="px-2.5" onclick={() => select('transcript')}>
							{m.watch_transcript()}
						</Tabs.Trigger>
					{/if}
				</Tabs.List>
				<Button
					variant="ghost"
					size="icon-sm"
					class="ms-auto text-muted-foreground hover:text-foreground"
					aria-expanded={open}
					aria-controls="{uid}-body"
					aria-label={open ? m.watch_panel_collapse() : m.watch_panel_expand()}
					title={open ? m.watch_panel_collapse() : m.watch_panel_expand()}
					onclick={toggle}
					data-testid="watch-panel-toggle"
				>
					<ChevronDownIcon class={cn('transition-transform', open && 'rotate-180')} />
				</Button>
			</header>
			<div id="{uid}-body" class="watch-panel-body flex min-h-0 flex-col border-t" hidden={!open}>
				{#if open && shown === 'chapters'}
					<Tabs.Content value="chapters" class="flex min-h-0 flex-1 flex-col">
						<ChapterList {chapters} current={currentChapter} {onseek} class="min-h-0 flex-1" />
					</Tabs.Content>
				{:else if open}
					<Tabs.Content value="transcript" class="flex h-[min(28rem,60svh)] min-h-0 flex-col">
						<TranscriptPanel
							videoId={video.id}
							{tracks}
							{time}
							{showing}
							{audioLang}
							{onseek}
							class="min-h-0 flex-1"
						/>
					</Tabs.Content>
				{/if}
			</div>
		</Tabs.Root>
	</section>
{/if}

<style>
	.watch-panel-body {
		max-height: min(28rem, 60svh);
	}

	.watch-panel-body[hidden] {
		display: none;
	}
</style>
