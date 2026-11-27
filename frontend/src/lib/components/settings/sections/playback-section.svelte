<script lang="ts">
	import {
		createSettingsMutation,
		createSettingsQuery,
		DEFAULT_SETTINGS
	} from '#lib/api/settings.js';
	import { session } from '#lib/session.svelte.js';
	import { guestAudioLanguage } from '#lib/settings/audio-language.svelte.js';
	import PlaybackSettings from '../playback-settings.svelte';
	import SettingsError from './settings-error.svelte';

	const settings = createSettingsQuery();
	const save = createSettingsMutation();
</script>

{#if session.user}
	<PlaybackSettings
		settings={settings.data ?? DEFAULT_SETTINGS}
		disabled={settings.isPending}
		onchange={(patch) => save.mutate(patch)}
	/>
	<SettingsError failed={save.isError} />
{:else}
	<PlaybackSettings
		settings={{ ...DEFAULT_SETTINGS, audio_language: guestAudioLanguage.current }}
		guest
		onchange={(patch) => {
			if (patch.audio_language !== undefined) guestAudioLanguage.set(patch.audio_language);
		}}
	/>
{/if}
