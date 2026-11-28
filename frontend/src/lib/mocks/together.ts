import type { RoomParticipant, RoomQueueEntry } from '#lib/together/connection.svelte.js';
import { mockVideos } from './videos';

function entry(index: number, addedBy: string): RoomQueueEntry {
	const v = mockVideos[index];
	return {
		id: 100 + index,
		added_by: addedBy,
		autoplay: false,
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
	{
		id: 'p-mara',
		name: 'mara',
		is_owner: true,
		is_guest: false,
		sync: 'synced',
		place: 'watching',
		status_revision: 3
	},
	{
		id: 'p-jonas',
		name: 'jonas',
		is_owner: false,
		is_guest: false,
		sync: 'buffering',
		place: 'browsing',
		status_revision: 5
	},
	{
		id: 'p-grace',
		name: 'Grace',
		is_owner: false,
		is_guest: true,
		sync: 'behind',
		place: 'shorts',
		status_revision: 2
	},
	{
		id: 'p-ada',
		name: 'Ada Lovelace-Byron of Ockham',
		is_owner: false,
		is_guest: true,
		sync: null,
		place: null,
		status_revision: 0
	}
];
