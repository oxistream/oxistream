<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import SearchIcon from '@lucide/svelte/icons/search';
	import XIcon from '@lucide/svelte/icons/x';
	import type { SubscribedChannel } from '#lib/api/subscription-list.js';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { chipActiveClass, chipClass } from '#lib/components/filter-chip.js';
	import * as Popover from '#lib/components/ui/popover/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import { matchesChannel } from './feed-filters.js';

	let {
		hideWatched,
		hideShorts,
		channel,
		channels,
		onhidewatched,
		onhideshorts,
		onchannel,
		class: className
	}: {
		hideWatched: boolean;
		hideShorts: boolean;
		/** ID of the only channel shown. */
		channel?: string;
		channels: SubscribedChannel[];
		onhidewatched: (hide: boolean) => void;
		onhideshorts: (hide: boolean) => void;
		onchannel: (id: string | undefined) => void;
		class?: string;
	} = $props();

	let open = $state(false);
	let search = $state('');

	const selected = $derived(channels.find((c) => c.id === channel));
	const selectedName = $derived(channel ? (selected?.name ?? channel) : undefined);
	const listed = $derived(channels.filter((c) => matchesChannel(c, search)));
	const active = $derived(hideWatched || hideShorts || channel !== undefined);

	function pick(id: string | undefined) {
		open = false;
		search = '';
		onchannel(id);
	}

	function clear() {
		if (hideWatched) onhidewatched(false);
		if (hideShorts) onhideshorts(false);
		if (channel) onchannel(undefined);
	}

	const option =
		'flex w-full min-w-0 items-center gap-2 rounded-sm px-2 py-1.5 text-start text-sm outline-none hover:bg-accent focus-visible:bg-accent';
</script>

<div
	role="group"
	aria-label={m.feed_filters_label()}
	class={cn('flex flex-wrap items-center gap-2', className)}
	data-testid="feed-filters"
>
	<button
		type="button"
		class={cn(chipClass, hideWatched && chipActiveClass)}
		aria-pressed={hideWatched}
		onclick={() => onhidewatched(!hideWatched)}
		data-testid="feed-filter-watched"
	>
		{m.feed_filter_hide_watched()}
	</button>
	<button
		type="button"
		class={cn(chipClass, hideShorts && chipActiveClass)}
		aria-pressed={hideShorts}
		onclick={() => onhideshorts(!hideShorts)}
		data-testid="feed-filter-shorts"
	>
		{m.feed_filter_hide_shorts()}
	</button>
	<Popover.Root bind:open onOpenChange={(next) => !next && (search = '')}>
		<Popover.Trigger
			class={cn(chipClass, 'max-w-64', selectedName && chipActiveClass)}
			aria-label={`${m.feed_filter_channel()}: ${selectedName ?? m.feed_filter_all_channels()}`}
			data-testid="feed-filter-channel"
		>
			{#if selectedName}
				<ChannelAvatar name={selectedName} src={selected?.avatarUrl} class="-ms-1.5 size-5" />
				<span class="truncate">{selectedName}</span>
			{:else}
				{m.feed_filter_channel()}
			{/if}
			<ChevronDownIcon class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
		</Popover.Trigger>
		<Popover.Content align="start" class="flex w-[min(18rem,calc(100vw-2rem))] flex-col gap-1 p-1">
			<label class="relative block">
				<span class="sr-only">{m.feed_filter_channel_search()}</span>
				<SearchIcon
					class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
					aria-hidden="true"
				/>
				<input
					type="search"
					bind:value={search}
					placeholder={m.feed_filter_channel_search()}
					class="h-8 w-full rounded-sm bg-transparent ps-8 pe-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:bg-muted"
					data-testid="feed-filter-channel-search"
				/>
			</label>
			<ul class="max-h-72 overflow-y-auto overscroll-contain" data-testid="feed-filter-channels">
				{#if !search}
					<li>
						<button
							type="button"
							class={option}
							aria-pressed={!channel}
							onclick={() => pick(undefined)}
						>
							<span class="flex-1 truncate">{m.feed_filter_all_channels()}</span>
							{#if !channel}<CheckIcon class="size-4 shrink-0" aria-hidden="true" />{/if}
						</button>
					</li>
				{/if}
				{#each listed as item (item.id)}
					{@const name = item.name ?? item.id}
					<li>
						<button
							type="button"
							class={option}
							aria-pressed={item.id === channel}
							onclick={() => pick(item.id)}
						>
							<ChannelAvatar {name} src={item.avatarUrl} size="sm" />
							<span class="flex-1 truncate">{name}</span>
							{#if item.id === channel}<CheckIcon class="size-4 shrink-0" aria-hidden="true" />{/if}
						</button>
					</li>
				{:else}
					<li class="px-2 py-3 text-center text-sm text-muted-foreground">
						{m.feed_filter_no_channels()}
					</li>
				{/each}
			</ul>
		</Popover.Content>
	</Popover.Root>
	{#if active}
		<button
			type="button"
			class={cn(chipClass, 'bg-transparent px-2 text-muted-foreground hover:text-foreground')}
			onclick={clear}
			data-testid="feed-filters-clear"
		>
			<XIcon class="size-3.5" aria-hidden="true" />
			{m.search_filters_clear()}
		</button>
	{/if}
</div>
