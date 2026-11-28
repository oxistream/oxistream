<script lang="ts">
	import {
		createClearHistoryMutation,
		createHistoryPausedMutation,
		createHistoryRetentionMutation,
		createHistorySettingsQuery
	} from '#lib/api/history.js';
	import { session } from '#lib/session.svelte.js';
	import HistorySettings from '../history-settings.svelte';
	import SettingsError from './settings-error.svelte';

	const settings = createHistorySettingsQuery();
	const pause = createHistoryPausedMutation();
	const retention = createHistoryRetentionMutation();
	const clear = createClearHistoryMutation();
</script>

<HistorySettings
	paused={settings.data?.paused === true}
	retention={settings.data?.retention}
	maxRetention={session.historyMaxRetention}
	disabled={settings.isPending}
	cleared={clear.isSuccess}
	onpause={(paused) => pause.mutate(paused)}
	onretention={(value) => retention.mutate(value)}
	onclear={() => clear.mutate()}
/>
<SettingsError failed={pause.isError || retention.isError || clear.isError} />
