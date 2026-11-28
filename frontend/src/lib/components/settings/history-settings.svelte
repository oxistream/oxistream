<script lang="ts">
	import { resolve } from '$app/paths';
	import { RETENTIONS, retentionOptions, type Retention } from '#lib/api/retention.js';
	import ClearHistoryDialog from '#lib/components/history/clear-history-dialog.svelte';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import SettingRow from './setting-row.svelte';

	let {
		paused,
		retention = 'forever',
		maxRetention = 'forever',
		disabled = false,
		cleared = false,
		onpause,
		onretention,
		onclear
	}: {
		paused: boolean;
		retention?: Retention;
		/** Longer retentions are not offered. */
		maxRetention?: Retention;
		disabled?: boolean;
		/** Shows a confirmation after clearing. */
		cleared?: boolean;
		onpause: (paused: boolean) => void;
		onretention?: (retention: Retention) => void;
		onclear: () => void;
	} = $props();

	const LABELS: Record<Retention, () => string> = {
		forever: m.retention_forever,
		'365d': m.retention_365d,
		'180d': m.retention_180d,
		'90d': m.retention_90d,
		'30d': m.retention_30d,
		'14d': m.retention_14d,
		'7d': m.retention_7d,
		'1d': m.retention_1d
	};
	const options = $derived(retentionOptions(maxRetention));
	// A stored value above the maximum acts as the maximum.
	const value = $derived(options.includes(retention) ? retention : options[0]);

	function isRetention(value: string): value is Retention {
		return (RETENTIONS as readonly string[]).includes(value);
	}
</script>

<SettingRow
	id="settings-history-paused"
	label={m.history_pause()}
	description={m.history_paused_body()}
>
	{#snippet control()}
		<Switch
			id="settings-history-paused"
			checked={paused}
			{disabled}
			aria-describedby="settings-history-paused-description"
			onCheckedChange={(checked) => onpause(checked)}
		/>
	{/snippet}
</SettingRow>

<SettingRow id="settings-history-retention" label={m.settings_history_retention()}>
	{#snippet control()}
		<Select.Root
			type="single"
			{value}
			{disabled}
			onValueChange={(next) => {
				if (isRetention(next) && next !== value) onretention?.(next);
			}}
		>
			<Select.Trigger id="settings-history-retention" class="w-44">
				{LABELS[value]()}
			</Select.Trigger>
			<Select.Content>
				{#each options as option (option)}
					<Select.Item value={option} label={LABELS[option]()} />
				{/each}
			</Select.Content>
		</Select.Root>
	{/snippet}
</SettingRow>

<SettingRow label={m.history_clear()} description={m.settings_history_clear_hint()}>
	{#snippet control()}
		<div class="flex items-center gap-3">
			{#if cleared}
				<span class="text-sm text-muted-foreground" role="status">
					{m.settings_history_cleared()}
				</span>
			{/if}
			<ClearHistoryDialog {disabled} onconfirm={onclear} />
		</div>
	{/snippet}
	<a
		href={resolve('/feed/history')}
		class="w-fit text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
	>
		{m.settings_history_open()}
	</a>
</SettingRow>
