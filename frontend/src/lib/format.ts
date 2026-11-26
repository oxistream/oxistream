/** Compact count such as "1.2M" (en) or "1,2 Mio." (de). */
export function formatCompactNumber(value: number, locale: string): string {
	const magnitude = value >= 1000 ? Math.floor(Math.log10(value) / 3) : 0;
	const scaled = value / 1000 ** magnitude;
	// One decimal only while the leading part is a single digit, as in "1.2M" but "12M".
	return new Intl.NumberFormat(locale, {
		notation: 'compact',
		maximumFractionDigits: scaled < 10 ? 1 : 0
	}).format(value);
}

/** Full grouped count such as "12,345" (en) or "12.345" (de). */
export function formatNumber(value: number, locale: string): string {
	return new Intl.NumberFormat(locale).format(value);
}

function splitSeconds(totalSeconds: number) {
	const total = Math.max(0, Math.floor(totalSeconds));
	return {
		hours: Math.floor(total / 3600),
		minutes: Math.floor((total % 3600) / 60),
		seconds: total % 60
	};
}

/** Player-style timestamp: "4:05", "1:02:03". */
export function formatDuration(totalSeconds: number): string {
	const { hours, minutes, seconds } = splitSeconds(totalSeconds);
	const ss = String(seconds).padStart(2, '0');
	if (hours > 0) return `${hours}:${String(minutes).padStart(2, '0')}:${ss}`;
	return `${minutes}:${ss}`;
}

type DurationFormatCtor = new (
	locale: string,
	options: { style: 'long' }
) => { format(duration: Record<string, number>): string };

/** Spoken duration for assistive tech: "4 minutes, 5 seconds" / "4 Minuten, 5 Sekunden". */
export function formatDurationLong(totalSeconds: number, locale: string): string {
	const { hours, minutes, seconds } = splitSeconds(totalSeconds);
	const DurationFormat = (Intl as unknown as { DurationFormat?: DurationFormatCtor })
		.DurationFormat;
	// DurationFormat drops zero units, so an all-zero duration takes the manual path.
	if (DurationFormat && hours + minutes + seconds > 0) {
		return new DurationFormat(locale, { style: 'long' }).format({ hours, minutes, seconds });
	}
	const unit = (value: number, name: string) =>
		new Intl.NumberFormat(locale, { style: 'unit', unit: name, unitDisplay: 'long' }).format(value);
	const out: string[] = [];
	if (hours) out.push(unit(hours, 'hour'));
	if (minutes) out.push(unit(minutes, 'minute'));
	if (seconds || out.length === 0) out.push(unit(seconds, 'second'));
	return new Intl.ListFormat(locale, { style: 'long', type: 'unit' }).format(out);
}

const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
	['year', 365 * 24 * 3600],
	['month', 30 * 24 * 3600],
	['week', 7 * 24 * 3600],
	['day', 24 * 3600],
	['hour', 3600],
	['minute', 60],
	['second', 1]
];

/** "3 weeks ago" / "vor 3 Wochen"; "yesterday" / "gestern" where the locale has a word for it. */
export function formatRelativeTime(
	date: Date | string,
	locale: string,
	now: Date = new Date()
): string {
	const then = typeof date === 'string' ? new Date(date) : date;
	const diffSeconds = Math.round((then.getTime() - now.getTime()) / 1000);
	const abs = Math.abs(diffSeconds);
	const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
	if (abs < 45) return rtf.format(0, 'second');
	for (const [unit, size] of RELATIVE_UNITS) {
		if (abs >= size) return rtf.format(Math.trunc(diffSeconds / size), unit);
	}
	return rtf.format(0, 'second');
}

/** Absolute date for tooltips: "March 4, 2026" / "4. März 2026". */
export function formatDate(date: Date | string, locale: string): string {
	const value = typeof date === 'string' ? new Date(date) : date;
	return new Intl.DateTimeFormat(locale, { dateStyle: 'long' }).format(value);
}
