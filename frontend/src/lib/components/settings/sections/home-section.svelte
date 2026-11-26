<script lang="ts">
	import { createSettingsMutation, createSettingsQuery } from '#lib/api/settings.js';
	import HomeSettings from '../home-settings.svelte';
	import SettingsError from './settings-error.svelte';

	const settings = createSettingsQuery();
	const save = createSettingsMutation();
</script>

<HomeSettings
	sections={settings.data?.home_sections ?? null}
	disabled={settings.isPending}
	onchange={(patch) => save.mutate(patch)}
/>
<SettingsError failed={save.isError} />
