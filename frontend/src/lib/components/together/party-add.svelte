<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { createSearchQuery, resolveYouTubeUrl } from '#lib/api/search.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { parseYouTubeUrl } from '#lib/youtube.js';
	import type { VideoSummary } from '#lib/video.js';
	import PartyPick from './party-pick.svelte';

	const RESULTS = 5;
	const DEBOUNCE_MS = 300;
	const CONFIRM_MS = 1500;

	let {
		queued,
		onadd
	}: {
		/** Video IDs already playing or queued. */
		queued: Set<string>;
		onadd: (videoId: string) => void;
	} = $props();

	const uid = $props.id();
	let query = $state('');
	let debounced = $state('');
	let resolving = $state(false);
	let notice = $state<'queued' | 'not_video' | null>(null);
	/** Picked here, before the room confirms them. */
	let sent = $state(new Set<string>());

	// Once the room reports its queue again, it tells what is queued.
	$effect(() => {
		void queued;
		sent = new Set();
	});

	const isLink = $derived(parseYouTubeUrl(query) !== null);
	$effect(() => {
		const q = query.trim();
		const timer = setTimeout(() => (debounced = q), DEBOUNCE_MS);
		return () => clearTimeout(timer);
	});
	const search = createSearchQuery(
		() => (isLink ? '' : debounced),
		() => 'videos'
	);
	const results = $derived(
		(search.data?.pages[0]?.items ?? [])
			// Livestreams and premieres cannot play in a party.
			.flatMap((item) =>
				item.type === 'video' && !item.video.isLive && !item.video.isUpcoming ? [item.video] : []
			)
			.slice(0, RESULTS)
	);
	const showResults = $derived(!isLink && query.trim().length > 0 && debounced.length > 0);

	$effect(() => {
		if (!notice) return;
		const timer = setTimeout(() => (notice = null), CONFIRM_MS);
		return () => clearTimeout(timer);
	});

	function pick(video: VideoSummary) {
		sent = new Set([...sent, video.id]);
		onadd(video.id);
	}

	async function addLink(text: string) {
		const link = parseYouTubeUrl(text);
		if (!link || resolving) return false;
		resolving = true;
		try {
			const target = await resolveYouTubeUrl(link.href);
			if (target.kind !== 'video') {
				notice = 'not_video';
				return true;
			}
			onadd(target.id);
			query = '';
			notice = 'queued';
		} catch {
			notice = 'not_video';
		} finally {
			resolving = false;
		}
		return true;
	}

	function onpaste(event: ClipboardEvent) {
		const text = event.clipboardData?.getData('text') ?? '';
		// Only a pasted link replacing the whole field is queued right away.
		if (query.trim() || !parseYouTubeUrl(text)) return;
		event.preventDefault();
		query = text.trim();
		void addLink(text);
	}

	function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		if (isLink) void addLink(query);
		else if (results[0] && !queued.has(results[0].id) && !sent.has(results[0].id)) {
			pick(results[0]);
		}
	}
</script>

<form role="search" class="flex flex-col gap-1" {onsubmit} data-testid="party-add">
	<div class="relative">
		{#if resolving}
			<LoaderCircleIcon
				class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 animate-spin text-muted-foreground"
				aria-hidden="true"
			/>
		{:else if notice === 'queued'}
			<CheckIcon
				class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-primary"
				aria-hidden="true"
			/>
		{:else}
			<SearchIcon
				class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
				aria-hidden="true"
			/>
		{/if}
		<Input
			type="search"
			bind:value={query}
			autocomplete="off"
			autocapitalize="off"
			spellcheck="false"
			enterkeyhint="search"
			placeholder={m.party_add_placeholder()}
			aria-label={m.party_add_label()}
			aria-controls={showResults ? `${uid}-results` : undefined}
			aria-invalid={notice === 'not_video' || undefined}
			aria-busy={resolving}
			class="h-8 bg-background ps-8 dark:bg-input/30"
			{onpaste}
			onkeydown={(event) => {
				if (event.key === 'Escape' && query) {
					event.preventDefault();
					query = '';
				}
			}}
			data-testid="party-add-input"
		/>
	</div>
	<p class="sr-only" role="status">
		{notice === 'queued'
			? m.party_add_queued()
			: notice === 'not_video'
				? m.party_add_not_video()
				: ''}
	</p>
	{#if notice === 'not_video'}
		<p class="px-1 text-xs font-medium text-destructive" aria-hidden="true">
			{m.party_add_not_video()}
		</p>
	{/if}
	{#if showResults}
		<div id="{uid}-results" class="flex flex-col" data-testid="party-add-results">
			{#if search.isPending}
				{#each { length: 3 }, i (i)}
					<div class="flex items-center gap-2 p-1">
						<Skeleton class="aspect-video w-20 rounded-md" />
						<Skeleton class="h-3 flex-1" />
					</div>
				{/each}
			{:else if search.isError}
				<p class="px-1 py-1 text-xs text-muted-foreground">{m.party_add_failed()}</p>
			{:else if results.length === 0}
				<p class="px-1 py-1 text-xs text-muted-foreground">{m.party_add_none()}</p>
			{:else}
				<ul class="flex flex-col">
					{#each results as video (video.id)}
						<li>
							<PartyPick
								{video}
								queued={queued.has(video.id) || sent.has(video.id)}
								onpick={pick}
							/>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</form>
