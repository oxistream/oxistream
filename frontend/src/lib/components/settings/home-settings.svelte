<script lang="ts">
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import { moveItem } from '#lib/api/playlist-order.js';
	import type { SettingsPatch } from '#lib/api/settings.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { editableSections, type HomeSectionKey } from '#lib/home/sections.js';
	import { m } from '#lib/paraglide/messages.js';
	import ReorderList from './reorder-list.svelte';
	import SettingRow from './setting-row.svelte';

	let {
		sections: stored,
		disabled = false,
		onchange
	}: {
		/** The stored layout; `null` for the default. */
		sections: string[] | null;
		disabled?: boolean;
		onchange: (patch: SettingsPatch) => void;
	} = $props();

	const LABELS: Record<HomeSectionKey, () => string> = {
		'continue-watching': m.home_continue_watching,
		subscriptions: m.home_latest_subscriptions,
		'subscription-shorts': m.home_subscription_shorts,
		'trending-news': m.settings_home_trending_news,
		'trending-gaming': m.settings_home_trending_gaming,
		'trending-podcasts': m.settings_home_trending_podcasts,
		'trending-music': m.settings_home_trending_music,
		'trending-sports': m.settings_home_trending_sports,
		'trending-learning': m.settings_home_trending_learning,
		'trending-fashion': m.settings_home_trending_fashion,
		'trending-live': m.settings_home_trending_live
	};

	const all = $derived(
		editableSections(stored).map((s) => ({ ...s, id: s.key, label: LABELS[s.key]() }))
	);
	const shown = $derived(all.filter((s) => s.enabled));
	const hidden = $derived(all.filter((s) => !s.enabled));

	const save = (keys: HomeSectionKey[]) => onchange({ home_sections: keys });
	const keys = (items: { key: HomeSectionKey }[]) => items.map((s) => s.key);
</script>

<SettingRow label={m.settings_home_shown()} description={m.settings_home_shown_hint()} stacked>
	{#snippet control()}
		<Button
			variant="outline"
			size="sm"
			disabled={disabled || stored === null}
			onclick={() => onchange({ home_sections: null })}
		>
			<RotateCcwIcon />
			{m.settings_reset_default()}
		</Button>
	{/snippet}
	{#if shown.length > 0}
		<ReorderList
			items={shown}
			label={m.settings_home_shown()}
			{disabled}
			class="max-w-xl"
			onmove={(from, to) => save(keys(moveItem(shown, from, to)))}
		>
			{#snippet trailing(item)}
				<Switch
					checked
					{disabled}
					aria-label={m.settings_home_toggle({ label: item.label })}
					onCheckedChange={() => save(keys(shown.filter((s) => s.key !== item.key)))}
				/>
			{/snippet}
		</ReorderList>
	{:else}
		<p class="max-w-xl rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">
			{m.settings_home_empty()}
		</p>
	{/if}
</SettingRow>

{#if hidden.length > 0}
	<SettingRow label={m.settings_home_hidden()} stacked>
		<ul class="flex max-w-xl flex-col gap-1" aria-label={m.settings_home_hidden()}>
			{#each hidden as item (item.key)}
				<li class="flex items-center gap-2 rounded-lg border border-dashed py-2 ps-3 pe-2">
					<span class="min-w-0 flex-1 text-sm text-pretty text-muted-foreground">{item.label}</span>
					<Switch
						checked={false}
						{disabled}
						aria-label={m.settings_home_toggle({ label: item.label })}
						onCheckedChange={() => save([...keys(shown), item.key])}
					/>
				</li>
			{/each}
		</ul>
	</SettingRow>
{/if}
