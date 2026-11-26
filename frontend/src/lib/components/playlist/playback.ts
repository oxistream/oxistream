import type { PlaylistDetails, PlaylistVisibility } from '#lib/api/playlists.js';
import type { LoadedYouTubePlaylist } from '#lib/api/youtube-playlists.js';
import type { VideoSummary } from '#lib/video.js';

/** What the watch page panel plays through: a local playlist or a (partly loaded) YouTube one. */
export type PlaybackList = {
	id: string;
	title: string;
	owner?: string;
	/** Only local playlists have a visibility. */
	visibility?: PlaylistVisibility;
	items: VideoSummary[];
	/** May exceed `items` while more pages can be loaded. */
	total: number;
};

export function localPlayback(playlist: PlaylistDetails): PlaybackList {
	return {
		id: playlist.id,
		title: playlist.title,
		owner: playlist.owner,
		visibility: playlist.visibility,
		items: playlist.items,
		total: playlist.items.length
	};
}

/** YouTube's count includes hidden unavailable videos, so it only holds until all are loaded. */
export function youtubePlayback(playlist: LoadedYouTubePlaylist, hasMore: boolean): PlaybackList {
	return {
		id: playlist.id,
		title: playlist.title,
		owner: playlist.channel?.name,
		items: playlist.items,
		total: hasMore ? Math.max(playlist.videoCount, playlist.items.length) : playlist.items.length
	};
}
