import { readJson, writeJson } from './storage';

export const PREFERENCES_KEY = 'oxistream:player';

export type PlayerPreferences = {
	volume?: number;
	muted?: boolean;
	playbackRate?: number;
	/** Language of the last caption track shown, or "off". */
	captions?: string;
};

function isPreferences(value: unknown): value is PlayerPreferences {
	if (typeof value !== 'object' || value === null || Array.isArray(value)) return false;
	const v = value as Record<string, unknown>;
	return (
		(v.volume === undefined || (typeof v.volume === 'number' && v.volume >= 0 && v.volume <= 1)) &&
		(v.muted === undefined || typeof v.muted === 'boolean') &&
		(v.playbackRate === undefined ||
			(typeof v.playbackRate === 'number' && v.playbackRate > 0 && v.playbackRate <= 4)) &&
		(v.captions === undefined || typeof v.captions === 'string')
	);
}

export function readPreferences(): PlayerPreferences {
	return readJson(PREFERENCES_KEY, isPreferences) ?? {};
}

export function savePreferences(patch: PlayerPreferences): void {
	writeJson(PREFERENCES_KEY, { ...readPreferences(), ...patch });
}
