import { describe, expect, it } from 'vitest';
import { isActive } from './navigation';

describe('isActive', () => {
	it('matches home only on the root path', () => {
		expect(isActive('/', '/')).toBe(true);
		expect(isActive('/feed/history', '/')).toBe(false);
	});

	it('matches a section and its subpaths', () => {
		expect(isActive('/settings', '/settings')).toBe(true);
		expect(isActive('/settings/account', '/settings')).toBe(true);
	});

	it('does not match paths that only share a prefix', () => {
		expect(isActive('/settingsx', '/settings')).toBe(false);
	});
});
