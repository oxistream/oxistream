import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import type { PlaybackPreferences } from '#lib/components/player/playback.js';
import { session, type ApiError as ErrorBody } from '#lib/session.svelte.js';
import { api } from './client';
import { unwrap } from './mapping';
import type { components } from './schema';

export type Settings = components['schemas']['Settings'];
export type SettingsPatch = components['schemas']['SettingsPatch'];
export type VideoCodec = components['schemas']['VideoCodec'];

export const VIDEO_CODECS: readonly VideoCodec[] = ['av1', 'vp9', 'h264'];
/** Mirrors `QUALITIES` in the backend's settings API. */
export const QUALITIES = [4320, 2160, 1440, 1080, 720, 480, 360, 240, 144] as const;

export const settingsKeys = { all: ['settings'] as const };

export const DEFAULT_SETTINGS: Settings = {
	video_codecs: [],
	max_quality: null,
	default_quality: null,
	home_sections: null
};

/** The signed-in user's settings; idle for guests, who always get the defaults. */
export function createSettingsQuery() {
	return createQuery(() => ({
		queryKey: settingsKeys.all,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/settings', { signal })),
		enabled: session.user !== null,
		staleTime: 5 * 60_000,
		// Pages wait for it; on failure they go on with the defaults.
		retry: 1
	}));
}

/** Saves a partial change, showing it right away and rolling back on failure. */
export function createSettingsMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (patch: SettingsPatch) =>
			unwrap(await api.PATCH('/api/v0/settings', { body: patch })),
		onMutate: async (patch: SettingsPatch) => {
			await client.cancelQueries({ queryKey: settingsKeys.all });
			const previous = client.getQueryData<Settings>(settingsKeys.all);
			client.setQueryData<Settings>(settingsKeys.all, {
				...(previous ?? DEFAULT_SETTINGS),
				...patch
			});
			return { previous };
		},
		onError: (_error: Error, _patch: SettingsPatch, context?: { previous?: Settings }) => {
			client.setQueryData(settingsKeys.all, context?.previous);
		},
		onSuccess: (settings: Settings) => client.setQueryData(settingsKeys.all, settings)
	}));
}

export function playbackPreferences(settings: Settings | undefined): PlaybackPreferences {
	return {
		codecs: settings?.video_codecs ?? [],
		maxQuality: settings?.max_quality ?? null,
		defaultQuality: settings?.default_quality ?? null
	};
}

export type AccountResult =
	| { ok: true }
	/** `error` is `null` when the server could not be reached or sent no error body. */
	| { ok: false; status: number; error: ErrorBody | null };

export async function changePassword(body: {
	current_password: string;
	new_password: string;
}): Promise<AccountResult> {
	try {
		const { error, response } = await api.POST('/api/v0/account/password', { body });
		return response.ok
			? { ok: true }
			: { ok: false, status: response.status, error: error ?? null };
	} catch {
		return { ok: false, status: 0, error: null };
	}
}

export async function deleteAccount(body: {
	username: string;
	password?: string;
}): Promise<AccountResult> {
	try {
		const { error, response } = await api.DELETE('/api/v0/account', { body });
		return response.ok
			? { ok: true }
			: { ok: false, status: response.status, error: error ?? null };
	} catch {
		return { ok: false, status: 0, error: null };
	}
}
