const UNITS = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+(?:\.\d+)?)s?)?$/;
const CLOCK = /^(?:(\d+):)?(\d{1,2}):(\d{1,2}(?:\.\d+)?)$/;

/**
 * Parses a YouTube-style `t` parameter: "90", "90s", "1m30s", "1h2m3s" or "1:02:03".
 * Returns whole seconds, or null when the value is empty or malformed.
 */
export function parseStartTime(value: string | null | undefined): number | null {
	const input = value?.trim().toLowerCase();
	if (!input) return null;

	const clock = CLOCK.exec(input);
	if (clock) {
		const [, h, m, s] = clock;
		return Math.floor(Number(h ?? 0) * 3600 + Number(m) * 60 + Number(s));
	}

	const units = UNITS.exec(input);
	if (!units || units.slice(1).every((part) => part === undefined)) return null;
	const [, h, m, s] = units;
	return Math.floor(Number(h ?? 0) * 3600 + Number(m ?? 0) * 60 + Number(s ?? 0));
}
