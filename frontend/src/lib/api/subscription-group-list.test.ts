import { describe, expect, it } from 'vitest';
import {
	isGroupId,
	isNameTaken,
	withFields,
	withGroupMoved,
	withMembership,
	type SubscriptionGroup
} from './subscription-group-list';

const group = (id: string, name: string, channelIds: string[] = []): SubscriptionGroup => ({
	id,
	name,
	showOnHome: false,
	channelIds
});

describe('subscription groups', () => {
	const groups = [group('a', 'Tech', ['UC1']), group('b', 'Music'), group('c', 'News')];

	it('adds and removes channels once', () => {
		const added = withMembership(groups, 'a', 'UC2', true);
		expect(added[0].channelIds).toEqual(['UC1', 'UC2']);
		expect(withMembership(added, 'a', 'UC2', true)[0].channelIds).toEqual(['UC1', 'UC2']);
		expect(withMembership(added, 'a', 'UC1', false)[0].channelIds).toEqual(['UC2']);
		expect(added[1]).toBe(groups[1]);
	});

	it('changes only the given fields', () => {
		const changed = withFields(groups, 'b', { showOnHome: true });
		expect(changed[1]).toEqual({ ...groups[1], showOnHome: true });
		expect(withFields(changed, 'b', { name: 'Songs' })[1].showOnHome).toBe(true);
	});

	it('moves groups', () => {
		expect(withGroupMoved(groups, 'c', 0).map((g) => g.id)).toEqual(['c', 'a', 'b']);
		expect(withGroupMoved(groups, 'a', 9).map((g) => g.id)).toEqual(['b', 'c', 'a']);
		expect(withGroupMoved(groups, 'x', 0)).toEqual(groups);
	});

	it('compares names ignoring case', () => {
		expect(isNameTaken(groups, ' tech ')).toBe(true);
		expect(isNameTaken(groups, 'TECH', 'a')).toBe(false);
		expect(isNameTaken(groups, 'Gaming')).toBe(false);
	});

	it('recognizes group ids', () => {
		expect(isGroupId('0194f1c2-7d3a-7c4e-9b1a-1234567890ab')).toBe(true);
		expect(isGroupId('nope')).toBe(false);
		expect(isGroupId(undefined)).toBe(false);
	});
});
