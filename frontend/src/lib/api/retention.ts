import type { components } from './schema';

export type Retention = components['schemas']['Retention'];

/** Longest first. */
export const RETENTIONS: readonly Retention[] = [
	'forever',
	'365d',
	'180d',
	'90d',
	'30d',
	'14d',
	'7d',
	'1d'
];

const DAY_MS = 24 * 60 * 60 * 1000;

/** How long entries are kept in milliseconds; `null` for forever. */
export function retentionMs(retention: Retention): number | null {
	return retention === 'forever' ? null : Number.parseInt(retention, 10) * DAY_MS;
}

/** The retentions a user may pick under the instance maximum, longest first. */
export function retentionOptions(max: Retention): Retention[] {
	return RETENTIONS.slice(Math.max(0, RETENTIONS.indexOf(max)));
}
