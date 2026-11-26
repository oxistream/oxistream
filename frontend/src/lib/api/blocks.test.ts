import { describe, expect, it } from 'vitest';
import { toBlockedChannel, withoutChannel } from './blocks';

const video = (id: string, channelId: string) => ({ id, channel: { id: channelId, name: 'x' } });

describe('withoutChannel', () => {
	it('drops videos, playlists and the channel from search pages', () => {
		const data = {
			pages: [
				{
					items: [
						{ type: 'video', key: 'v:a', video: video('a', 'UCblocked') },
						{ type: 'video', key: 'v:b', video: video('b', 'UCother') },
						{ type: 'channel', key: 'c:x', channel: { id: 'UCblocked', name: 'Blocked' } },
						{ type: 'playlist', key: 'p:1', playlist: { id: '1', channel: { id: 'UCblocked' } } },
						{ type: 'playlist', key: 'p:2', playlist: { id: '2' } }
					],
					continuation: 'next'
				}
			],
			pageParams: [undefined]
		};
		const kept = withoutChannel(data, 'UCblocked');
		expect(kept.pages[0].items.map((item) => item.key)).toEqual(['v:b', 'p:2']);
		expect(kept.pages[0].continuation).toBe('next');
		expect(data.pages[0].items).toHaveLength(5);
	});

	it('keeps the watched video and drops its related videos of the channel', () => {
		const details = {
			...video('w', 'UCblocked'),
			related: [video('r1', 'UCblocked'), video('r2', 'UCother')]
		};
		const kept = withoutChannel(details, 'UCblocked');
		expect(kept.id).toBe('w');
		expect(kept.related.map((v) => v.id)).toEqual(['r2']);
	});

	it('leaves plain values alone', () => {
		expect(withoutChannel(['a', 1, null], 'UCblocked')).toEqual(['a', 1, null]);
		expect(withoutChannel(undefined, 'UCblocked')).toBeUndefined();
	});
});

describe('toBlockedChannel', () => {
	it('picks a small avatar', () => {
		expect(
			toBlockedChannel({
				id: 'UCa',
				name: 'Alpha',
				avatar: [
					{ url: '/big', width: 800, height: 800 },
					{ url: '/small', width: 88, height: 88 }
				],
				blocked_at: '2026-11-26T10:00:00Z'
			})
		).toEqual({ id: 'UCa', name: 'Alpha', avatarUrl: '/small', blockedAt: '2026-11-26T10:00:00Z' });
	});
});
