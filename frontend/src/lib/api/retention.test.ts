import { describe, expect, it } from 'vitest';
import { retentionMs, retentionOptions } from './retention';

describe('retention', () => {
	it('converts to milliseconds', () => {
		expect(retentionMs('forever')).toBeNull();
		expect(retentionMs('1d')).toBe(24 * 60 * 60 * 1000);
		expect(retentionMs('365d')).toBe(365 * 24 * 60 * 60 * 1000);
	});

	it('offers only retentions up to the maximum', () => {
		expect(retentionOptions('forever')).toHaveLength(8);
		expect(retentionOptions('30d')).toEqual(['30d', '14d', '7d', '1d']);
		expect(retentionOptions('1d')).toEqual(['1d']);
	});
});
