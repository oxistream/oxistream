import type { components } from '../api/schema';

export type SponsorBlockSettings = components['schemas']['SponsorBlockSettings'];
export type SkipCategories = components['schemas']['SkipCategories'];
export type SkipCategory = keyof SkipCategories;
export type SkipMode = components['schemas']['SkipMode'];
export type Segment = components['schemas']['Segment'];
export type SegmentCategory = components['schemas']['Category'];

export const SKIP_CATEGORIES: SkipCategory[] = [
	'sponsor',
	'selfpromo',
	'interaction',
	'intro',
	'outro',
	'preview',
	'hook',
	'music_offtopic',
	'filler'
];

const SKIP_MODES: SkipMode[] = ['auto', 'button', 'off'];

export function defaultSettings(): SponsorBlockSettings {
	return {
		categories: {
			sponsor: 'auto',
			selfpromo: 'button',
			interaction: 'button',
			intro: 'button',
			outro: 'button',
			preview: 'button',
			hook: 'button',
			music_offtopic: 'button',
			filler: 'button'
		},
		highlight: true,
		full_video_label: true
	};
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Valid values of `value` over the defaults, e.g. for settings stored by an older version. */
export function normalizeSettings(value: unknown): SponsorBlockSettings {
	const settings = defaultSettings();
	if (!isRecord(value)) return settings;
	const categories = isRecord(value.categories) ? value.categories : {};
	for (const category of SKIP_CATEGORIES) {
		const mode = categories[category];
		if (SKIP_MODES.includes(mode as SkipMode)) settings.categories[category] = mode as SkipMode;
	}
	if (typeof value.highlight === 'boolean') settings.highlight = value.highlight;
	if (typeof value.full_video_label === 'boolean')
		settings.full_video_label = value.full_video_label;
	return settings;
}

export function isSkipCategory(category: string): category is SkipCategory {
	return (SKIP_CATEGORIES as string[]).includes(category);
}
