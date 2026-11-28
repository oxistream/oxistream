<script lang="ts">
	import { setMode, userPrefersMode } from 'mode-watcher';
	import SegmentedControl from '#lib/components/segmented-control.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, setLocale, type Locale } from '#lib/paraglide/runtime.js';

	type Mode = 'light' | 'dark' | 'system';

	const themes: { value: Mode; label: string }[] = [
		{ value: 'system', label: m.theme_system() },
		{ value: 'light', label: m.theme_light() },
		{ value: 'dark', label: m.theme_dark() }
	];
	const languages: { value: Locale; label: string; lang: string }[] = [
		{ value: 'en', label: m.language_en(), lang: 'en' },
		{ value: 'de', label: m.language_de(), lang: 'de' }
	];
</script>

<div class="px-5 py-4">
	<SegmentedControl
		name="theme"
		legend={m.theme_label()}
		options={themes}
		value={userPrefersMode.current}
		onchange={(value) => setMode(value)}
	/>
</div>
<div class="px-5 py-4">
	<SegmentedControl
		name="language"
		legend={m.language_label()}
		options={languages}
		value={getLocale()}
		onchange={(value) => void setLocale(value)}
	/>
</div>
