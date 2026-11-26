// Typed stand-in data for stories and tests. Thumbnails are licensed photographs
// (see ./thumbnails/credits.ts), bundled locally so nothing is fetched from YouTube.
import type { SubscribedChannel } from '#lib/api/subscription-list.js';
import type { ChannelSummary, User, VideoSummary } from '#lib/video.js';
import anvil from './thumbnails/anvil.jpg';
import barista from './thumbnails/barista.jpg';
import bikeBell from './thumbnails/bike-bell.jpg';
import bikeChain from './thumbnails/bike-chain.jpg';
import bookbinding from './thumbnails/bookbinding.jpg';
import breadOven from './thumbnails/bread-oven.jpg';
import coffeeRoast from './thumbnails/coffee-roast.jpg';
import espresso from './thumbnails/espresso.jpg';
import faroeRoad from './thumbnails/faroe-road.jpg';
import florist from './thumbnails/florist.jpg';
import luthier from './thumbnails/luthier.jpg';
import stockfish from './thumbnails/stockfish.jpg';
import westfjordsRoad from './thumbnails/westfjords-road.jpg';
import westfjordsValley from './thumbnails/westfjords-valley.jpg';

const HOUR = 3600 * 1000;
const DAY = 24 * HOUR;

function ago(ms: number, now = Date.now()): string {
	return new Date(now - ms).toISOString();
}

export const mockChannels = {
	forge: {
		id: 'UCm0ck4nv1lF0rg3H4mm3r1',
		name: 'Hammer & Tongs',
		handle: '@hammerandtongs',
		avatarUrl: anvil
	},
	north: {
		id: 'UCm0ckN0rthb0und0R0ads2',
		name: 'Northbound',
		handle: '@northbound',
		avatarUrl: westfjordsValley
	},
	bench: {
		id: 'UCm0ckB3nchN0t3sLuth1r3',
		name: 'Bench Notes',
		handle: '@benchnotes',
		avatarUrl: luthier
	},
	crumb: {
		id: 'UCm0ckCrumbTh30ryBr3ad4',
		name: 'Crumb Theory',
		handle: '@crumbtheory',
		avatarUrl: breadOven
	},
	dial: { id: 'UCm0ckD1al1nC0ff33Lab05', name: 'Dial In', handle: '@dialin', avatarUrl: espresso },
	spoke: {
		id: 'UCm0ckSp0k3AndCha1nB1k6',
		name: 'Spoke & Chain',
		handle: '@spokeandchain',
		avatarUrl: bikeChain
	},
	bindery: { id: 'UCm0ckB1nd3ryH0urs0Bk7', name: 'Bindery Hours', handle: '@binderyhours' },
	stem: {
		id: 'UCm0ckSt3mAndTw1n3Fl0w8',
		name: 'Stem & Twine',
		handle: '@stemandtwine',
		avatarUrl: florist
	}
} satisfies Record<string, ChannelSummary>;

const c = mockChannels;

export const mockVideos: VideoSummary[] = [
	{
		id: 'k3Rv9fTq2Lw',
		title: 'Forging a bottle opener from an old horseshoe, start to finish',
		channel: c.forge,
		durationSeconds: 1394,
		viewCount: 482_310,
		publishedAt: ago(5 * HOUR),
		thumbnailUrl: anvil,
		progress: 0.42
	},
	{
		id: 'Wq8zP1mXc4E',
		title: 'Route 624 in the Westfjords: gravel, fog and nobody else',
		channel: c.north,
		durationSeconds: 2861,
		viewCount: 1_284_005,
		publishedAt: ago(2 * DAY),
		thumbnailUrl: westfjordsRoad,
		progress: 0.71
	},
	{
		id: 'Lm2nB7vQ0sA',
		title: 'Carving and fitting a violin top in its mould',
		channel: c.bench,
		durationSeconds: 3725,
		viewCount: 96_420,
		publishedAt: ago(4 * DAY),
		thumbnailUrl: luthier,
		progress: 0.18
	},
	{
		id: 'Tf5yH3kJd9R',
		title: 'Dialling in a light roast on a bottomless portafilter',
		channel: c.dial,
		durationSeconds: 754,
		viewCount: 213_880,
		publishedAt: ago(6 * DAY),
		thumbnailUrl: espresso,
		progress: 0.93
	},
	{
		id: 'Pz0aE6wNn1U',
		title: 'Rye and caraway loaves in a 100-year-old wood oven',
		channel: c.crumb,
		durationSeconds: 1822,
		viewCount: 57_019,
		publishedAt: ago(9 * HOUR),
		thumbnailUrl: breadOven
	},
	{
		id: 'Xc7uG2rLb8K',
		title: 'When to replace a chain and cassette (and when not to)',
		channel: c.spoke,
		durationSeconds: 642,
		viewCount: 734_102,
		publishedAt: ago(1 * DAY),
		thumbnailUrl: bikeChain
	},
	{
		id: 'Hj4dS9oVt3Y',
		title: 'Hand-gilding raised bands on a leather spine',
		channel: c.bindery,
		durationSeconds: 1105,
		viewCount: 18_644,
		publishedAt: ago(3 * DAY),
		thumbnailUrl: bookbinding
	},
	{
		id: 'Ne1qW6pZf7M',
		title: 'Conditioning stems so a bouquet lasts two weeks',
		channel: c.stem,
		durationSeconds: 487,
		viewCount: 129_360,
		publishedAt: ago(5 * DAY),
		thumbnailUrl: florist
	},
	{
		id: 'Bv3kR8yUa5Q',
		title: 'Faroe Islands by car: Viðareiði and the northern road',
		channel: c.north,
		durationSeconds: 2214,
		viewCount: 2_041_773,
		publishedAt: ago(8 * DAY),
		thumbnailUrl: faroeRoad
	},
	{
		id: 'Gy9mT0eIc2D',
		title: 'Reading a roast with the sample trier',
		channel: c.dial,
		durationSeconds: 931,
		viewCount: 44_208,
		publishedAt: ago(12 * DAY),
		thumbnailUrl: coffeeRoast
	},
	{
		id: 'Rs6oL4hWx0F',
		title: 'Stockfish season in Å i Lofoten',
		channel: c.north,
		durationSeconds: 1537,
		viewCount: 388_912,
		publishedAt: ago(18 * DAY),
		thumbnailUrl: stockfish
	},
	{
		id: 'Ua2fN5jPq6Z',
		title: 'Tamping is not the problem: five espresso myths tested',
		channel: c.dial,
		durationSeconds: 1268,
		viewCount: 902_457,
		publishedAt: ago(26 * DAY),
		thumbnailUrl: barista
	},
	{
		id: 'Ik8wC1vMy4T',
		title: 'A winter commute in Amsterdam, unedited',
		channel: c.spoke,
		durationSeconds: 4512,
		viewCount: 61_337,
		publishedAt: ago(41 * DAY),
		thumbnailUrl: bikeBell
	},
	{
		id: 'Oq3pX7tKe9B',
		title: 'A Westfjords valley, in fog and then sun',
		channel: c.north,
		durationSeconds: 368,
		viewCount: 7_903,
		publishedAt: ago(70 * DAY),
		thumbnailUrl: westfjordsValley
	}
];

export const mockContinueWatching = mockVideos.filter((video) => video.progress !== undefined);

export const mockUser: User = {
	id: 'usr_01',
	name: 'Mara Lindqvist',
	username: 'mara'
};

const subscriberCounts: Record<string, number> = { forge: 284_000, north: 1_240_000, dial: 52_300 };

export const mockSubscriptions: SubscribedChannel[] = Object.entries(mockChannels)
	.map(([key, channel]: [string, ChannelSummary]) => ({
		...channel,
		verified: key === 'north',
		subscriberCount: subscriberCounts[key],
		subscribedAt: '2026-11-01T10:00:00Z'
	}))
	.sort((a, b) => a.name.localeCompare(b.name));

/** A channel the feed worker has not fetched yet. */
export const mockPendingSubscription: SubscribedChannel = {
	id: 'UCm0ckP3nd1ngCh4nn3l000',
	verified: false,
	subscribedAt: '2026-11-26T10:00:00Z'
};
