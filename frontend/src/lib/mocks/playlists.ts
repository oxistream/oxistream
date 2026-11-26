import type { Playlist, PlaylistDetails } from '#lib/api/playlists.js';
import { mockVideos } from './videos';

/** Reference time for relative dates in stories. */
export const mockPlaylistNow = new Date('2026-11-26T12:00:00Z');

const items = mockVideos.slice(0, 8).map((video) => ({
	...video,
	addedAt: '2026-11-20T10:00:00Z'
}));

export const mockPlaylist: PlaylistDetails = {
	id: 'Rd7xQ2mKp9LsVb3N',
	title: 'North Atlantic road trips',
	description: 'Long drives through the Westfjords and the Faroes.\nWatch in order.',
	visibility: 'unlisted',
	itemCount: items.length,
	thumbnailUrl: items[0].thumbnailUrl,
	lastVideoId: items[2].id,
	createdAt: '2026-11-02T10:00:00Z',
	updatedAt: '2026-11-25T18:00:00Z',
	owner: 'mara',
	isOwner: true,
	items
};

export const mockPlaylists: Playlist[] = [
	mockPlaylist,
	{
		id: 'Nw4pL8sTq1ZcXy6M',
		title: 'Workshop techniques to try this winter',
		description: '',
		visibility: 'private',
		itemCount: 3,
		thumbnailUrl: mockVideos[5].thumbnailUrl,
		createdAt: '2026-10-12T10:00:00Z',
		updatedAt: '2026-11-19T09:00:00Z'
	},
	{
		id: 'Qm2vB7xKc4TzHs8P',
		title: 'Empty for now',
		description: '',
		visibility: 'private',
		itemCount: 0,
		createdAt: '2026-11-26T08:00:00Z',
		updatedAt: '2026-11-26T08:00:00Z'
	}
];
