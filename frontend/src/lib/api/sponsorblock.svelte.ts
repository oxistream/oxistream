import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { readJson, writeJson } from '#lib/components/player/storage.js';
import { session } from '#lib/session.svelte.js';
import {
	defaultSettings,
	normalizeSettings,
	type Segment,
	type SponsorBlockSettings
} from '#lib/sponsorblock/settings.js';
import { api } from './client';
import { unwrap } from './mapping';

export const GUEST_SETTINGS_KEY = 'oxistream:sponsorblock';

export const sponsorBlockKeys = {
	settings: ['sponsorblock', 'settings'] as const,
	segments: (id: string | null) => ['sponsorblock', 'segments', id] as const
};

/** Segments of a video; empty while loading, on errors and when the instance disables them. */
export function useSegments(id: () => string | null) {
	return createQuery(() => {
		const videoId = id();
		return {
			queryKey: sponsorBlockKeys.segments(videoId),
			queryFn: async ({ signal }: { signal: AbortSignal }): Promise<Segment[]> =>
				unwrap(
					await api.GET('/api/v0/videos/{id}/segments', {
						params: { path: { id: videoId! } },
						signal
					})
				).segments,
			enabled: videoId !== null && session.sponsorblock,
			staleTime: 10 * 60_000,
			retry: false
		};
	});
}

const isObject = (value: unknown): value is object => typeof value === 'object';

function readGuestSettings(): SponsorBlockSettings {
	return normalizeSettings(readJson(GUEST_SETTINGS_KEY, isObject));
}

// Shared, so a change in the preferences dialog reaches the player right away.
const guest = $state({ settings: readGuestSettings() });

/** The viewer's preferences: on the server when signed in, in this browser otherwise. */
export function useSponsorBlockSettings() {
	const client = useQueryClient();

	const server = createQuery(() => ({
		queryKey: sponsorBlockKeys.settings,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			normalizeSettings(unwrap(await api.GET('/api/v0/settings/sponsorblock', { signal }))),
		enabled: session.user !== null,
		staleTime: 5 * 60_000
	}));

	const save = createMutation(() => ({
		mutationFn: async (next: SponsorBlockSettings) =>
			normalizeSettings(unwrap(await api.PUT('/api/v0/settings/sponsorblock', { body: next }))),
		onMutate: async (next: SponsorBlockSettings) => {
			await client.cancelQueries({ queryKey: sponsorBlockKeys.settings });
			const previous = client.getQueryData<SponsorBlockSettings>(sponsorBlockKeys.settings);
			client.setQueryData(sponsorBlockKeys.settings, next);
			return { previous };
		},
		onError: (
			_error: Error,
			_next: SponsorBlockSettings,
			context?: { previous?: SponsorBlockSettings }
		) => {
			client.setQueryData(sponsorBlockKeys.settings, context?.previous);
		},
		onSuccess: (saved: SponsorBlockSettings) =>
			client.setQueryData(sponsorBlockKeys.settings, saved)
	}));

	return {
		get current(): SponsorBlockSettings {
			if (!session.user) return guest.settings;
			return server.data ?? defaultSettings();
		},
		/** Preferences are known, so the player can act on them. */
		get ready(): boolean {
			return session.ready && (!session.user || !server.isPending);
		},
		get signedIn(): boolean {
			return session.user !== null;
		},
		get saveFailed(): boolean {
			return save.isError;
		},
		update(next: SponsorBlockSettings) {
			if (session.user) {
				save.mutate(next);
			} else {
				guest.settings = next;
				writeJson(GUEST_SETTINGS_KEY, next);
			}
		}
	};
}

export type SponsorBlockSettingsStore = ReturnType<typeof useSponsorBlockSettings>;
