<script lang="ts">
	import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
	import { setMode, userPrefersMode } from 'mode-watcher';
	import PageHeader from '#lib/components/page-header.svelte';
	import SegmentedControl from '#lib/components/segmented-control.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
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

<div class="flex max-w-3xl flex-col gap-6 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_settings()} />

	<section
		aria-labelledby="appearance-heading"
		class="flex flex-col gap-6 rounded-xl border bg-card p-5"
	>
		<h2 id="appearance-heading" class="text-base font-semibold tracking-tight">
			{m.settings_appearance()}
		</h2>
		<SegmentedControl
			name="theme"
			legend={m.theme_label()}
			description={m.settings_appearance_hint()}
			options={themes}
			value={userPrefersMode.current}
			onchange={(value) => setMode(value)}
		/>
		<SegmentedControl
			name="language"
			legend={m.language_label()}
			description={m.settings_language_hint()}
			options={languages}
			value={getLocale()}
			onchange={(value) => void setLocale(value)}
		/>
	</section>

	<EmptyState icon={SlidersHorizontalIcon} title={m.settings_more()} />
</div>
