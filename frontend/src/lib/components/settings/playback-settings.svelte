<script lang="ts">
	import { moveItem } from '#lib/api/playlist-order.js';
	import {
		QUALITIES,
		VIDEO_CODECS,
		type Settings,
		type SettingsPatch,
		type VideoCodec
	} from '#lib/api/settings.js';
	import SegmentedControl from '#lib/components/segmented-control.svelte';
	import * as Select from '#lib/components/ui/select/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import {
		AUDIO_LANGUAGES,
		INTERFACE_LANGUAGE,
		languageName
	} from '#lib/settings/audio-language.svelte.js';
	import ReorderList from './reorder-list.svelte';
	import SettingRow from './setting-row.svelte';

	let {
		settings,
		disabled = false,
		guest = false,
		onchange
	}: {
		settings: Settings;
		disabled?: boolean;
		/** Only the settings a guest's browser keeps: the audio track. */
		guest?: boolean;
		onchange: (patch: SettingsPatch) => void;
	} = $props();

	const ORIGINAL = 'original';
	const audioOptions = $derived([
		{ value: ORIGINAL, label: m.settings_audio_original() },
		{
			value: INTERFACE_LANGUAGE,
			label: m.settings_audio_interface({ language: languageName(getLocale()) })
		},
		...AUDIO_LANGUAGES.map((code) => ({ value: code, label: languageName(code) })).sort((a, b) =>
			a.label.localeCompare(b.label, getLocale())
		)
	]);
	const audioValue = $derived(settings.audio_language ?? ORIGINAL);

	const CODEC_LABELS: Record<VideoCodec, string> = { av1: 'AV1', vp9: 'VP9', h264: 'H.264' };
	const AUTO = 'auto';

	const custom = $derived(settings.video_codecs.length > 0);
	// A custom order lists every codec, so unlisted ones keep their place at the end.
	const codecs = $derived(
		[
			...settings.video_codecs,
			...VIDEO_CODECS.filter((c) => !settings.video_codecs.includes(c))
		].map((id) => ({ id, label: CODEC_LABELS[id] }))
	);

	function qualityLabel(height: number): string {
		const names: Record<number, string> = { 4320: '8K', 2160: '4K', 1440: 'QHD', 1080: 'HD' };
		return names[height] ? `${height}p (${names[height]})` : `${height}p`;
	}
	const qualities = QUALITIES.map((height) => ({
		value: String(height),
		label: qualityLabel(height)
	}));
	const maxOptions = [{ value: AUTO, label: m.settings_quality_unlimited() }, ...qualities];
	const defaultOptions = [{ value: AUTO, label: m.settings_quality_auto() }, ...qualities];

	const parseQuality = (value: string) => (value === AUTO ? null : Number(value));
	const maxValue = $derived(settings.max_quality ? String(settings.max_quality) : AUTO);
	const defaultValue = $derived(settings.default_quality ? String(settings.default_quality) : AUTO);
	const labelOf = (options: { value: string; label: string }[], value: string) =>
		options.find((o) => o.value === value)?.label ?? value;
</script>

{#if !guest}
	<SettingRow label={m.settings_codec()} description={m.settings_codec_hint()} stacked>
		{#snippet control()}
			<SegmentedControl
				name="codec-mode"
				legend={m.settings_codec_mode()}
				class="[&>div]:mt-0 [&>legend]:sr-only"
				options={[
					{ value: 'auto', label: m.settings_codec_auto() },
					{ value: 'custom', label: m.settings_codec_custom() }
				]}
				value={custom ? 'custom' : 'auto'}
				onchange={(mode) =>
					onchange({ video_codecs: mode === 'custom' ? codecs.map((c) => c.id) : [] })}
			/>
		{/snippet}
		{#if custom}
			<ReorderList
				items={codecs}
				label={m.settings_codec_order()}
				{disabled}
				class="max-w-md"
				onmove={(from, to) =>
					onchange({ video_codecs: moveItem(codecs, from, to).map((c) => c.id) })}
			/>
		{/if}
	</SettingRow>

	<SettingRow
		id="settings-max-quality"
		label={m.settings_max_quality()}
		description={m.settings_max_quality_hint()}
	>
		{#snippet control()}
			<Select.Root
				type="single"
				value={maxValue}
				{disabled}
				onValueChange={(value) => onchange({ max_quality: parseQuality(value) })}
			>
				<Select.Trigger
					id="settings-max-quality"
					class="w-44"
					aria-describedby="settings-max-quality-description"
				>
					{labelOf(maxOptions, maxValue)}
				</Select.Trigger>
				<Select.Content>
					{#each maxOptions as option (option.value)}
						<Select.Item value={option.value} label={option.label} />
					{/each}
				</Select.Content>
			</Select.Root>
		{/snippet}
	</SettingRow>

	<SettingRow
		id="settings-default-quality"
		label={m.settings_default_quality()}
		description={m.settings_default_quality_hint()}
	>
		{#snippet control()}
			<Select.Root
				type="single"
				value={defaultValue}
				{disabled}
				onValueChange={(value) => onchange({ default_quality: parseQuality(value) })}
			>
				<Select.Trigger
					id="settings-default-quality"
					class="w-44"
					aria-describedby="settings-default-quality-description"
				>
					{labelOf(defaultOptions, defaultValue)}
				</Select.Trigger>
				<Select.Content>
					{#each defaultOptions as option (option.value)}
						<Select.Item value={option.value} label={option.label} />
					{/each}
				</Select.Content>
			</Select.Root>
		{/snippet}
	</SettingRow>
{/if}

<SettingRow
	id="settings-audio-language"
	label={m.settings_audio_language()}
	description={m.settings_audio_language_hint()}
>
	{#snippet control()}
		<Select.Root
			type="single"
			value={audioValue}
			{disabled}
			onValueChange={(value) => onchange({ audio_language: value === ORIGINAL ? null : value })}
		>
			<Select.Trigger
				id="settings-audio-language"
				class="w-56"
				aria-describedby="settings-audio-language-description"
				data-testid="settings-audio-language"
			>
				<span class="truncate">{labelOf(audioOptions, audioValue)}</span>
			</Select.Trigger>
			<Select.Content>
				{#each audioOptions as option (option.value)}
					<Select.Item value={option.value} label={option.label} />
				{/each}
			</Select.Content>
		</Select.Root>
	{/snippet}
</SettingRow>
