import type { RoomListItem } from '#lib/api/rooms.js';
import type { RoomParticipant, RoomQueueEntry } from '#lib/together/connection.svelte.js';
import { mockVideos } from './videos';

/** Reference time for relative dates in stories. */
export const mockRoomNow = new Date('2026-11-26T12:00:00Z');

function entry(index: number, addedBy: string): RoomQueueEntry {
	const v = mockVideos[index];
	return {
		id: 100 + index,
		added_by: addedBy,
		video: {
			id: v.id,
			title: v.title,
			duration_seconds: v.durationSeconds ?? null,
			thumbnails: v.thumbnailUrl ? [{ url: v.thumbnailUrl, width: 480, height: 270 }] : [],
			channel: {
				id: v.channel.id,
				name: v.channel.name,
				avatar: [],
				subscriber_count: null,
				verified: false
			},
			published_at: null,
			published_text: null,
			view_count: null,
			viewers: null,
			is_live: false,
			is_short: false,
			is_upcoming: false,
			description: null
		}
	};
}

export const mockRoomCurrent = entry(0, 'mara');
export const mockRoomQueue: RoomQueueEntry[] = [
	entry(1, 'Grace'),
	entry(2, 'mara'),
	entry(3, 'Ada')
];

export const mockParticipants: RoomParticipant[] = [
	{ id: 'p-mara', name: 'mara', is_owner: true, is_guest: false },
	{ id: 'p-jonas', name: 'jonas', is_owner: false, is_guest: false },
	{ id: 'p-grace', name: 'Grace', is_owner: false, is_guest: true },
	{ id: 'p-ada', name: 'Ada Lovelace-Byron of Ockham', is_owner: false, is_guest: true }
];

export const mockRooms: RoomListItem[] = [
	{
		id: 'Qm9vbS1vbmUtMTIzNDU2Nw',
		participantCount: 3,
		queueCount: 4,
		currentTitle: mockVideos[0].title,
		thumbnailUrl: mockVideos[0].thumbnailUrl,
		createdAt: '2026-11-26T10:30:00Z',
		activeAt: '2026-11-26T11:58:00Z'
	},
	{
		id: 'Um9vbS10d28tNzY1NDMyMQ',
		participantCount: 0,
		queueCount: 0,
		createdAt: '2026-11-25T19:00:00Z',
		activeAt: '2026-11-25T21:00:00Z'
	}
];
