import { m } from '#lib/paraglide/messages.js';
import type { SegmentCategory, SkipCategory } from './settings';

export function categoryLabel(category: SegmentCategory): string {
	switch (category) {
		case 'sponsor':
			return m.sponsorblock_category_sponsor();
		case 'selfpromo':
			return m.sponsorblock_category_selfpromo();
		case 'interaction':
			return m.sponsorblock_category_interaction();
		case 'intro':
			return m.sponsorblock_category_intro();
		case 'outro':
			return m.sponsorblock_category_outro();
		case 'preview':
			return m.sponsorblock_category_preview();
		case 'hook':
			return m.sponsorblock_category_hook();
		case 'music_offtopic':
			return m.sponsorblock_category_music_offtopic();
		case 'filler':
			return m.sponsorblock_category_filler();
		case 'poi_highlight':
			return m.sponsorblock_category_poi_highlight();
		case 'exclusive_access':
			return m.sponsorblock_category_exclusive_access();
	}
}

export function categoryHint(category: SkipCategory): string {
	switch (category) {
		case 'sponsor':
			return m.sponsorblock_category_sponsor_hint();
		case 'selfpromo':
			return m.sponsorblock_category_selfpromo_hint();
		case 'interaction':
			return m.sponsorblock_category_interaction_hint();
		case 'intro':
			return m.sponsorblock_category_intro_hint();
		case 'outro':
			return m.sponsorblock_category_outro_hint();
		case 'preview':
			return m.sponsorblock_category_preview_hint();
		case 'hook':
			return m.sponsorblock_category_hook_hint();
		case 'music_offtopic':
			return m.sponsorblock_category_music_offtopic_hint();
		case 'filler':
			return m.sponsorblock_category_filler_hint();
	}
}

/** CSS colour of a category, defined in `colors.css`. */
export function categoryColor(category: SegmentCategory): string {
	return `var(--sb-${category.replaceAll('_', '-')})`;
}
