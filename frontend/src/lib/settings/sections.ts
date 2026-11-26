import BanIcon from '@lucide/svelte/icons/ban';
import ClockIcon from '@lucide/svelte/icons/clock';
import HouseIcon from '@lucide/svelte/icons/house';
import PaletteIcon from '@lucide/svelte/icons/palette';
import PlayIcon from '@lucide/svelte/icons/play';
import UserIcon from '@lucide/svelte/icons/user';
import type { Component } from 'svelte';
import { resolve } from '$app/paths';
import AppearanceSettings from '#lib/components/settings/appearance-settings.svelte';
import AccountSection from '#lib/components/settings/sections/account-section.svelte';
import HistorySection from '#lib/components/settings/sections/history-section.svelte';
import HomeSection from '#lib/components/settings/sections/home-section.svelte';
import PlaybackSection from '#lib/components/settings/sections/playback-section.svelte';
import { m } from '#lib/paraglide/messages.js';

type Base = {
	/** Anchor in the settings page, e.g. `/settings#playback`. */
	id: string;
	title: () => string;
	description?: () => string;
	icon: Component;
	/** Only shown to signed-in users. */
	account: boolean;
};

/**
 * A settings page section: either rendered in place by a component that loads and saves its
 * own data, or a link to a page of its own.
 */
export type SettingsSection =
	| (Base & { component: Component; href?: never })
	| (Base & { href: () => string; component?: never });

/** In page order. A new feature adds its section here. */
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
		icon: ClockIcon,
		account: true,
		component: HistorySection
	},
	{
		id: 'blocked-channels',
		title: m.blocked_title,
		description: m.blocked_description,
		icon: BanIcon,
		account: true,
		href: () => resolve('/settings/blocked')
	},
	{
		id: 'appearance',
		title: m.settings_appearance,
		icon: PaletteIcon,
		account: false,
		component: AppearanceSettings
	},
	{
		id: 'account',
		title: m.settings_account,
		icon: UserIcon,
		account: true,
		component: AccountSection
	}
];

export function visibleSections(signedIn: boolean): SettingsSection[] {
	return SETTINGS_SECTIONS.filter((section) => signedIn || !section.account);
}
