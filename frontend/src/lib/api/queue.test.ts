import { describe, expect, it } from 'vitest';
import { fromQueue, toNewEntry } from './queue';

describe('queue mapping', () => {
	it('sends only what the server accepts', () => {
		expect(
			toNewEntry({
				key: 'k1',
				id: 'vvvvvvvvvv0',
				title: '  ',
				channel: { id: '', name: '' },
				durationSeconds: 41.6
			})
		).toEqual({
			key: 'k1',
			video_id: 'vvvvvvvvvv0',
			title: 'vvvvvvvvvv0',
			channel_id: null,
			channel_name: null,
			duration_seconds: 42
		});
		const channel = { id: 'UCaaaaaaaaaaaaaaaaaaaaaa', name: 'Alpha' };
		expect(toNewEntry({ key: 'k', id: 'v', title: 'T', channel })).toMatchObject({
			channel_id: channel.id,
			channel_name: 'Alpha',
			duration_seconds: null
		});
	});

	it('maps server entries', () => {
		const state = fromQueue({
			revision: 2,
			entries: [
				{
					key: 'k1',
					video: {
						id: 'v',
						title: 'T',
						thumbnails: [
							{ url: '/big', width: 480, height: 360 },
							{ url: '/small', width: 320, height: 180 }
						],
						channel: { id: 'UC1', name: 'Alpha', avatar: [], verified: false },
						duration_seconds: 42,
						is_live: false,
						is_short: false,
						is_upcoming: false
					}
				}
			]
		});
		expect(state).toEqual({
			revision: 2,
			entries: [
				{
					key: 'k1',
					id: 'v',
					title: 'T',
					channel: { id: 'UC1', name: 'Alpha' },
					thumbnailUrl: '/small',
					durationSeconds: 42
				}
			]
		});
	});
});
