import { describe, expect, it } from 'vitest';
import { matchesChannel } from './feed-filters';

describe('channel filter search', () => {
	const channel = { id: 'UCaaaaaaaaaaaaaaaaaaaaaa', name: 'Café Nørd', handle: '@cafenord' };

	it('matches name and handle, ignoring case and accents', () => {
		expect(matchesChannel(channel, '')).toBe(true);
		expect(matchesChannel(channel, '  cafe ')).toBe(true);
		expect(matchesChannel(channel, 'CAFÉ')).toBe(true);
		expect(matchesChannel(channel, '@cafen')).toBe(true);
		expect(matchesChannel(channel, 'tea')).toBe(false);
	});

	it('falls back to the ID while the name is unknown', () => {
		expect(matchesChannel({ id: 'UCxyz' }, 'ucx')).toBe(true);
	});
});
