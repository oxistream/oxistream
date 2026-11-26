// Stand-in watch page data for stories, shaped like the API's VideoDetails.
import type { ApiVideoSummary, VideoDetails } from '#lib/api/video.js';
import { mockVideos } from '#lib/mocks/videos.js';
import type { RelatedVideo } from './related';

const thumb = (url?: string) => (url ? [{ url, width: 640, height: 360 }] : []);

export const mockNow = new Date('2026-11-26T12:00:00Z');

const forge = mockVideos[0];

export const mockDescription = `A horseshoe from the scrap bin becomes a bottle opener in one afternoon: heat, draw out, punch, file and finish.

Chapters
0:00 Picking a shoe
1:42 Drawing out the handle
6:18 Punching the opener
14:05 Filing and finish
23:10 First bottle

Tools: the 1.5 kg cross-peen and a homemade hardy, see https://example.org/hardy-build (part one: https://youtu.be/k3Rv9fTq2Lw?t=95).
Steel and coal from the local co-op. Questions? Ask below.`;

export const mockVideoDetails: VideoDetails = {
	id: forge.id,
	title: forge.title,
	description: mockDescription,
	channel: {
		id: forge.channel.id,
		name: forge.channel.name,
		avatar: thumb(forge.channel.avatarUrl),
		subscriber_count: 284_000,
		verified: true
	},
	view_count: forge.viewCount,
	like_count: 18_420,
	published_at: '2026-11-21T09:30:00Z',
	published_text: 'Nov 21, 2026',
	duration_seconds: forge.durationSeconds,
	is_live: false,
	keywords: ['blacksmithing', 'forging'],
	thumbnails: thumb(forge.thumbnailUrl),
	subtitles: [
		{ id: 'en', lang: 'en', label: 'English', auto_generated: false, url: '' },
		{ id: 'a.de', lang: 'de', label: 'German', auto_generated: true, url: '' }
	],
	related: mockVideos.slice(1, 9).map((v): ApiVideoSummary => ({
		id: v.id,
		title: v.title,
		thumbnails: thumb(v.thumbnailUrl),
		channel: { id: v.channel.id, name: v.channel.name, avatar: [], verified: false },
		duration_seconds: v.durationSeconds,
		view_count: v.viewCount,
		published_at: v.publishedAt,
		is_live: false,
		is_short: false,
		is_upcoming: false
	}))
};

export const mockRelated: RelatedVideo[] = mockVideos.slice(1).map((v, i) => ({
	id: v.id,
	title: v.title,
	thumbnailUrl: v.thumbnailUrl,
	durationSeconds: v.durationSeconds,
	// YouTube often sends related items without channel, views or date; every third one here.
	...(i % 3 === 2
		? {}
		: {
				viewCount: v.viewCount,
				publishedAt: v.publishedAt,
				channel: { id: v.channel.id, name: v.channel.name }
			})
}));
