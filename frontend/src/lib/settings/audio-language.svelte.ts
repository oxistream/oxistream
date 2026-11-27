import { readJson, writeJson } from '#lib/components/player/storage.js';
import { getLocale } from '#lib/paraglide/runtime.js';

export const GUEST_AUDIO_LANGUAGE_KEY = 'oxistream:audio-language';
/** Follows the interface language instead of a fixed one. */
export const INTERFACE_LANGUAGE = 'interface';

/** Offered in the settings; the API accepts any language code. */
export const AUDIO_LANGUAGES = [
	'ar',
	'bn',
	'de',
	'en',
	'es',
	'fr',
	'hi',
	'id',
	'it',
	'ja',
	'ko',
	'nl',
	'pl',
	'pt',
	'ru',
	'th',
	'tr',
	'uk',
	'vi',
	'zh'
] as const;

const PATTERN = /^([a-z]{2,3}(-[A-Za-z0-9]{2,8})?|interface)$/;

export function isAudioLanguage(value: unknown): value is string {
	return typeof value === 'string' && PATTERN.test(value);
}

// Shared, so a change in the settings reaches the next player right away.
const guest = $state({ language: readJson(GUEST_AUDIO_LANGUAGE_KEY, isAudioLanguage) });

/** A guest's preference, kept in this browser; `null` for the original audio. */
export const guestAudioLanguage = {
	get current(): string | null {
		return guest.language;
	},
	set(value: string | null) {
		guest.language = value;
		writeJson(GUEST_AUDIO_LANGUAGE_KEY, value);
	}
};

/** The language code the player looks for, or `null` for the original. */
export function resolveAudioLanguage(value: string | null | undefined): string | null {
	if (!value) return null;
	return value === INTERFACE_LANGUAGE ? getLocale() : value;
}

/** A language's name in the interface language, e.g. "German" or "Deutsch". */
export function languageName(code: string): string {
	try {
		return new Intl.DisplayNames([getLocale()], { type: 'language' }).of(code) ?? code;
	} catch {
		return code;
	}
}
