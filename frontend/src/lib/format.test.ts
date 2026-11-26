import { describe, expect, it } from 'vitest';
import {
	formatCompactNumber,
	formatDate,
	formatDuration,
	formatDurationLong,
	formatNumber,
	formatRelativeDay,
	formatRelativeTime
} from './format';

// Intl inserts narrow and regular no-break spaces; compare on plain spaces.
const plain = (value: string) => value.replace(/[\u00a0\u202f]/g, ' ');

describe('formatCompactNumber', () => {
	it('abbreviates in English', () => {
		expect(formatCompactNumber(999, 'en')).toBe('999');
		expect(formatCompactNumber(1234, 'en')).toBe('1.2K');
		expect(formatCompactNumber(12_345, 'en')).toBe('12K');
		expect(formatCompactNumber(1_234_567, 'en')).toBe('1.2M');
		expect(formatCompactNumber(12_345_678, 'en')).toBe('12M');
	});

	it('abbreviates in German', () => {
		expect(plain(formatCompactNumber(1_234_567, 'de'))).toBe('1,2 Mio.');
		expect(plain(formatCompactNumber(2_345_678_901, 'de'))).toBe('2,3 Mrd.');
	});
});

describe('formatNumber', () => {
	it('groups digits per locale', () => {
		expect(formatNumber(12_345, 'en')).toBe('12,345');
		expect(formatNumber(12_345, 'de')).toBe('12.345');
	});
});

describe('formatDuration', () => {
	it('formats minutes and seconds', () => {
		expect(formatDuration(0)).toBe('0:00');
		expect(formatDuration(65)).toBe('1:05');
		expect(formatDuration(599.9)).toBe('9:59');
	});

	it('adds hours when needed', () => {
		expect(formatDuration(3723)).toBe('1:02:03');
		expect(formatDuration(36_000)).toBe('10:00:00');
	});

	it('clamps negative input', () => {
		expect(formatDuration(-5)).toBe('0:00');
	});
});

describe('formatDurationLong', () => {
	it('spells out units in English and German', () => {
		expect(formatDurationLong(245, 'en')).toBe('4 minutes, 5 seconds');
		expect(formatDurationLong(245, 'de')).toBe('4 Minuten, 5 Sekunden');
		expect(formatDurationLong(3600, 'en')).toBe('1 hour');
	});

	it('handles zero', () => {
		expect(formatDurationLong(0, 'en')).toBe('0 seconds');
		expect(formatDurationLong(0, 'de')).toBe('0 Sekunden');
	});
});

describe('formatRelativeTime', () => {
	const now = new Date('2026-11-26T12:00:00Z');

	it('picks the largest fitting unit in English', () => {
		expect(formatRelativeTime('2026-11-26T11:59:30Z', 'en', now)).toBe('now');
		expect(formatRelativeTime('2026-11-26T11:55:00Z', 'en', now)).toBe('5 minutes ago');
		expect(formatRelativeTime('2026-11-25T10:00:00Z', 'en', now)).toBe('yesterday');
		expect(formatRelativeTime('2026-11-05T12:00:00Z', 'en', now)).toBe('3 weeks ago');
		expect(formatRelativeTime('2026-08-01T12:00:00Z', 'en', now)).toBe('3 months ago');
		expect(formatRelativeTime('2024-05-01T12:00:00Z', 'en', now)).toBe('2 years ago');
	});

	it('localizes to German', () => {
		expect(formatRelativeTime('2026-11-26T09:00:00Z', 'de', now)).toBe('vor 3 Stunden');
		expect(formatRelativeTime('2026-11-25T10:00:00Z', 'de', now)).toBe('gestern');
		expect(formatRelativeTime(new Date('2026-11-20T12:00:00Z'), 'de', now)).toBe('vor 6 Tagen');
	});
});

describe('formatRelativeDay', () => {
	const now = new Date(2026, 10, 26, 23, 30);

	it('counts calendar days, not hours', () => {
		expect(formatRelativeDay('2026-11-26', 'en', now)).toBe('today');
		expect(formatRelativeDay('2026-11-25', 'en', now)).toBe('yesterday');
		expect(formatRelativeDay('2026-11-22', 'en', now)).toBe('4 days ago');
		expect(formatRelativeDay('2026-11-05', 'en', now)).toBe('3 weeks ago');
		expect(formatRelativeDay('2024-05-01', 'en', now)).toBe('2 years ago');
		expect(formatRelativeDay('2026-11-27', 'en', now)).toBe('today');
	});

	it('localizes to German', () => {
		expect(formatRelativeDay('2026-11-26', 'de', now)).toBe('heute');
		expect(formatRelativeDay('2026-11-24', 'de', now)).toBe('vorgestern');
		expect(formatRelativeDay('2026-08-01', 'de', now)).toBe('vor 3 Monaten');
	});
});

describe('formatDate', () => {
	it('uses the long date style', () => {
		expect(formatDate('2026-03-04T12:00:00Z', 'en')).toBe('March 4, 2026');
		expect(formatDate('2026-03-04T12:00:00Z', 'de')).toBe('4. März 2026');
	});
});
