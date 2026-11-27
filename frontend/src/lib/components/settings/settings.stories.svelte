<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { DEFAULT_SETTINGS, type Settings, type SettingsPatch } from '#lib/api/settings.js';
	import type { Account } from '#lib/session.svelte.js';
	import AccountSettings from './account-settings.svelte';
	import AppearanceSettings from './appearance-settings.svelte';
	import HistorySettings from './history-settings.svelte';
	import HomeSettings from './home-settings.svelte';
	import PlaybackSettings from './playback-settings.svelte';
	import ReorderList from './reorder-list.svelte';
	import SettingsSection from './settings-section.svelte';

	const { Story } = defineMeta({
		title: 'Settings/Sections',
		component: SettingsSection
	});

	const local: Account = {
		id: '00000000-0000-0000-0000-000000000001',
		username: 'ada',
		role: 'admin',
		has_password: true,
		created_at: '2026-11-01T12:00:00Z'
	};
	const sso: Account = { ...local, username: 'grace', role: 'user', has_password: false };
	const wait = () => new Promise((resolve) => setTimeout(resolve, 400));
</script>

<script lang="ts">
	let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
	const update = (patch: SettingsPatch) => (settings = { ...settings, ...patch });
	let paused = $state(false);
	let codecs = $state([
		{ id: 'av1', label: 'AV1' },
		{ id: 'vp9', label: 'VP9' },
		{ id: 'h264', label: 'H.264' }
	]);
</script>

<Story name="Playback" asChild>
	<SettingsSection id="playback" title="Playback" class="max-w-3xl">
		<PlaybackSettings {settings} onchange={update} />
	</SettingsSection>
</Story>

<Story name="Playback, custom codecs" asChild>
	<SettingsSection id="playback" title="Playback" class="max-w-3xl">
		<PlaybackSettings
			settings={{ ...DEFAULT_SETTINGS, video_codecs: ['vp9', 'av1'], max_quality: 1080 }}
			onchange={() => {}}
		/>
	</SettingsSection>
</Story>

<Story name="Playback, guest" asChild>
	<SettingsSection id="playback" title="Playback" class="max-w-3xl">
		<PlaybackSettings
			settings={{ ...DEFAULT_SETTINGS, audio_language: 'interface' }}
			guest
			onchange={() => {}}
		/>
	</SettingsSection>
</Story>

<Story name="Home page" asChild>
	<SettingsSection id="home" title="Home page" class="max-w-3xl">
		<HomeSettings sections={settings.home_sections} onchange={update} />
	</SettingsSection>
</Story>

<Story name="Home page, all hidden" asChild>
	<SettingsSection id="home" title="Home page" class="max-w-3xl">
		<HomeSettings sections={[]} onchange={() => {}} />
	</SettingsSection>
</Story>

<Story name="History" asChild>
	<SettingsSection id="history" title="History" class="max-w-3xl">
		<HistorySettings {paused} onpause={(p) => (paused = p)} onclear={() => {}} />
	</SettingsSection>
</Story>

<Story name="Appearance" asChild>
	<SettingsSection id="appearance" title="Appearance" class="max-w-3xl">
		<AppearanceSettings />
	</SettingsSection>
</Story>

<Story name="Local account" asChild>
	<SettingsSection id="account" title="Account" class="max-w-3xl">
		<AccountSettings
			account={local}
			onchangepassword={async () => {
				await wait();
				return { ok: true };
			}}
			ondelete={async () => {
				await wait();
				return {
					ok: false,
					status: 409,
					error: { code: 'last_admin', message: 'last admin' }
				};
			}}
		/>
	</SettingsSection>
</Story>

<Story name="Single sign-on account" asChild>
	<SettingsSection id="account" title="Account" class="max-w-3xl">
		<AccountSettings
			account={sso}
			onchangepassword={() => Promise.resolve({ ok: true })}
			ondelete={() => Promise.resolve({ ok: true })}
		/>
	</SettingsSection>
</Story>

<Story name="Reorder list" asChild>
	<ReorderList
		items={codecs}
		label="Codecs"
		class="max-w-md"
		onmove={(from, to) => {
			const next = [...codecs];
			const [item] = next.splice(from, 1);
			next.splice(to, 0, item);
			codecs = next;
		}}
	/>
</Story>
