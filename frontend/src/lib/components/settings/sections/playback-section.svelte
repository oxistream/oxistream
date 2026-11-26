<script lang="ts">
	import {
		createSettingsMutation,
		createSettingsQuery,
		DEFAULT_SETTINGS
	} from '#lib/api/settings.js';
	import PlaybackSettings from '../playback-settings.svelte';
	import SettingsError from './settings-error.svelte';

	const settings = createSettingsQuery();
	const save = createSettingsMutation();
</script>

<PlaybackSettings
	settings={settings.data ?? DEFAULT_SETTINGS}
	disabled={settings.isPending}
	onchange={(patch) => save.mutate(patch)}
/>
<SettingsError failed={save.isError} />
