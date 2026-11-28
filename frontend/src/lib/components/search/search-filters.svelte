<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import XIcon from '@lucide/svelte/icons/x';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { chipActiveClass, chipClass as chip } from '#lib/components/filter-chip.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		DURATIONS,
		filtersFor,
		SEARCH_SORTS,
		searchParams,
		UPLOAD_DATES,
		type SearchFilter,
		type SearchFilters,
		type SearchType
	} from '#lib/search-types.js';
	import { cn } from '#lib/utils.js';

	let {
		query,
		type,
		filters,
		class: className
	}: { query: string; type: SearchType; filters: SearchFilters; class?: string } = $props();

	type Option = { value: string; label: () => string };
	type Group = { name: () => string; any: () => string; options: Option[] };

	const groups: Record<SearchFilter, Group> = {
		upload_date: {
			name: m.search_filter_upload_date,
			any: m.search_upload_date_any,
			options: UPLOAD_DATES.map((value) => ({
				value,
				label: {
					hour: m.search_upload_date_hour,
					day: m.search_upload_date_day,
					week: m.search_upload_date_week,
					month: m.search_upload_date_month,
					year: m.search_upload_date_year
				}[value]
			}))
		},
		duration: {
			name: m.search_filter_duration,
			any: m.search_duration_any,
			options: DURATIONS.map((value) => ({
				value,
				label: {
					short: m.search_duration_short,
					medium: m.search_duration_medium,
					long: m.search_duration_long
				}[value]
			}))
		},
		sort: {
			name: m.search_filter_sort,
			any: m.search_sort_relevance,
			options: SEARCH_SORTS.filter((value) => value !== 'relevance').map((value) => ({
				value,
				label: { date: m.search_sort_date, views: m.search_sort_views }[value]
			}))
		}
	};

	const offered = $derived(filtersFor(type));
	const active = $derived(offered.some((key) => filters[key] !== undefined));

	function apply(next: SearchFilters) {
		void goto(resolve(`/results?${searchParams(query, type, next)}`), { reset: false });
	}

	function set(key: SearchFilter, value: string) {
		apply({ ...filters, [key]: value === '' ? undefined : value });
	}
</script>

{#if offered.length > 0}
	<div
		role="group"
		aria-label={m.search_filters_label()}
		class={cn('flex flex-wrap items-center gap-2', className)}
		data-testid="search-filters"
	>
		{#each offered as key (key)}
			{@const group = groups[key]}
			{@const value = filters[key]}
			{@const label = group.options.find((o) => o.value === value)?.label()}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger
					class={cn(chip, label && chipActiveClass)}
					aria-label={`${group.name()}: ${label ?? group.any()}`}
					data-testid="search-filter-{key}"
				>
					{label ?? group.name()}
					<ChevronDownIcon class="size-3.5 opacity-70" aria-hidden="true" />
				</DropdownMenu.Trigger>
				<DropdownMenu.Content class="w-auto min-w-44">
					<DropdownMenu.Label>{group.name()}</DropdownMenu.Label>
					<DropdownMenu.RadioGroup value={value ?? ''} onValueChange={(next) => set(key, next)}>
						<DropdownMenu.RadioItem value="" closeOnSelect>{group.any()}</DropdownMenu.RadioItem>
						{#each group.options as option (option.value)}
							<DropdownMenu.RadioItem value={option.value} closeOnSelect>
								{option.label()}
							</DropdownMenu.RadioItem>
						{/each}
					</DropdownMenu.RadioGroup>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/each}
		{#if active}
			<button
				type="button"
				class={cn(chip, 'bg-transparent px-2 text-muted-foreground hover:text-foreground')}
				onclick={() => apply({})}
				data-testid="search-filters-clear"
			>
				<XIcon class="size-3.5" aria-hidden="true" />
				{m.search_filters_clear()}
			</button>
		{/if}
	</div>
{/if}
