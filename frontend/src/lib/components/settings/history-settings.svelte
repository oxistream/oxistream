<script lang="ts">
	import { resolve } from '$app/paths';
	import ClearHistoryDialog from '#lib/components/history/clear-history-dialog.svelte';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import SettingRow from './setting-row.svelte';

	let {
		paused,
		disabled = false,
		cleared = false,
		onpause,
		onclear
	}: {
		paused: boolean;
		disabled?: boolean;
		/** Shows a confirmation after clearing. */
		cleared?: boolean;
		onpause: (paused: boolean) => void;
		onclear: () => void;
	} = $props();
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
