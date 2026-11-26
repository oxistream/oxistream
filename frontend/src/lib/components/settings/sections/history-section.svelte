<script lang="ts">
	import {
		createClearHistoryMutation,
		createHistoryPausedMutation,
		createHistorySettingsQuery
	} from '#lib/api/history.js';
	import HistorySettings from '../history-settings.svelte';
	import SettingsError from './settings-error.svelte';

	const settings = createHistorySettingsQuery();
	const pause = createHistoryPausedMutation();
	const clear = createClearHistoryMutation();
</script>

<HistorySettings
	paused={settings.data === true}
	disabled={settings.isPending}
	cleared={clear.isSuccess}
	onpause={(paused) => pause.mutate(paused)}
	onclear={() => clear.mutate()}
/>
<SettingsError failed={pause.isError || clear.isError} />
