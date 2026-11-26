import BanIcon from '@lucide/svelte/icons/ban';
import ClockIcon from '@lucide/svelte/icons/clock';
import HouseIcon from '@lucide/svelte/icons/house';
import PaletteIcon from '@lucide/svelte/icons/palette';
import PlayIcon from '@lucide/svelte/icons/play';
import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
import UserIcon from '@lucide/svelte/icons/user';
import type { Component } from 'svelte';
import { resolve } from '$app/paths';
import AppearanceSettings from '#lib/components/settings/appearance-settings.svelte';
import AccountSection from '#lib/components/settings/sections/account-section.svelte';
import HistorySection from '#lib/components/settings/sections/history-section.svelte';
import HomeSection from '#lib/components/settings/sections/home-section.svelte';
import PlaybackSection from '#lib/components/settings/sections/playback-section.svelte';
import SponsorBlockSection from '#lib/components/settings/sections/sponsorblock-section.svelte';
import { m } from '#lib/paraglide/messages.js';
import { session } from '#lib/session.svelte.js';

type Base = {
	/** Route segment: the section lives at `/settings/{id}`. */
	id: string;
	title: () => string;
	description: () => string;
	icon: Component;
	/** Only shown to signed-in users. */
	account: boolean;
	/** Hidden while this returns false, e.g. for a feature the server has turned off. */
	available?: () => boolean;
};

/**
 * A settings section, listed on the settings overview, its navigation and the sidebar. Either
 * rendered at `/settings/{id}` by a component that loads and saves its own data, or a page of
 * its own at `href`.
 */
export type SettingsSection =
	| (Base & { component: Component; href?: never })
	| (Base & { href: () => string; component?: never });

/** In navigation order. A new feature adds its section here. */
export const SETTINGS_SECTIONS: SettingsSection[] = [
	{
		id: 'playback',
		title: m.settings_playback,
		description: m.settings_playback_hint,
		icon: PlayIcon,
		account: true,
		component: PlaybackSection
	},
	{
		id: 'home',
		title: m.settings_home,
		description: m.settings_home_hint,
		icon: HouseIcon,
		account: true,
		component: HomeSection
	},
	{
		id: 'history',
		title: m.nav_history,
		description: m.settings_history_hint,
		icon: ClockIcon,
		account: true,
		component: HistorySection
	},
	{
		id: 'sponsorblock',
		title: m.sponsorblock_title,
		description: m.settings_sponsorblock_hint,
		icon: SkipForwardIcon,
		account: false,
		available: () => session.sponsorblock,
		component: SponsorBlockSection
	},
	{
		id: 'blocked',
		title: m.blocked_title,
		description: m.blocked_description,
		icon: BanIcon,
		account: true,
		href: () => resolve('/settings/blocked')
	},
	{
		id: 'appearance',
		title: m.settings_appearance,
		description: m.settings_appearance_description,
		icon: PaletteIcon,
		account: false,
		component: AppearanceSettings
	},
	{
		id: 'account',
		title: m.settings_account,
		description: m.settings_account_hint,
		icon: UserIcon,
		account: true,
		component: AccountSection
	}
];

export function visibleSections(signedIn: boolean): SettingsSection[] {
	return SETTINGS_SECTIONS.filter(
		(section) => (signedIn || !section.account) && (section.available?.() ?? true)
	);
}

export function sectionHref(section: SettingsSection): string {
	return section.href?.() ?? resolve('/settings/[section]', { section: section.id });
}

/** A section rendered at `/settings/{id}`. */
export function findSection(id: string): (SettingsSection & { component: Component }) | undefined {
	return SETTINGS_SECTIONS.find(
		(section): section is SettingsSection & { component: Component } =>
			section.id === id && section.component !== undefined
	);
}
