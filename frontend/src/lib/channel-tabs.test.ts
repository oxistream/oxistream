import { describe, expect, it } from 'vitest';
import {
	apiTabSegment,
	channelTabHref,
	isChannelTab,
	parseChannelSort,
	parseChannelTab,
	pickPortrait,
	visibleTabs
} from './channel-tabs';

describe('channel tab parsing', () => {
	it('knows YouTube tab names and defaults to videos', () => {
		expect(isChannelTab('streams')).toBe(true);
		expect(isChannelTab('live')).toBe(false);
		expect(parseChannelTab(undefined)).toBe('videos');
		expect(parseChannelTab('shorts')).toBe('shorts');
		expect(parseChannelTab('community')).toBe('videos');
	});

	it('falls back to the latest sort', () => {
		expect(parseChannelSort('popular')).toBe('popular');
		expect(parseChannelSort('random')).toBe('latest');
		expect(parseChannelSort(null)).toBe('latest');
	});

	it('maps the live tab to the API segment', () => {
		expect(apiTabSegment('streams')).toBe('live');
		expect(apiTabSegment('shorts')).toBe('shorts');
	});
});

describe('channelTabHref', () => {
	it('builds YouTube-style tab URLs', () => {
		expect(channelTabHref('/@forge', 'shorts')).toBe('/@forge/shorts');
		expect(channelTabHref('/channel/UC1', 'playlists')).toBe('/channel/UC1/playlists');
	});

	it('keeps only a non-default sort on video tabs', () => {
		expect(channelTabHref('/@forge', 'videos', { sort: 'latest' })).toBe('/@forge/videos');
		expect(channelTabHref('/@forge', 'streams', { sort: 'oldest' })).toBe(
			'/@forge/streams?sort=oldest'
		);
		expect(channelTabHref('/@forge', 'playlists', { sort: 'popular' })).toBe('/@forge/playlists');
	});

	it('puts the search query in `query`', () => {
		expect(channelTabHref('/@forge', 'search', { query: ' hot cut ' })).toBe(
			'/@forge/search?query=hot+cut'
		);
		expect(channelTabHref('/@forge', 'search', { query: '  ' })).toBe('/@forge/search');
	});
});

describe('visibleTabs', () => {
	it('hides shorts and live when the channel has none', () => {
		expect(visibleTabs({ hasShorts: false, hasLive: false }, 'videos')).toEqual([
			'videos',
			'playlists'
		]);
		expect(visibleTabs({ hasShorts: true, hasLive: true }, 'search')).toEqual([
			'videos',
			'shorts',
			'streams',
			'playlists'
		]);
	});

	it('keeps a hidden tab that was opened directly', () => {
		expect(visibleTabs({ hasShorts: false, hasLive: false }, 'streams')).toEqual([
			'videos',
			'streams',
			'playlists'
		]);
	});
});

describe('pickPortrait', () => {
	it('prefers the most upright, then the largest image', () => {
		const images = [
			{ url: 'a', width: 404, height: 606 },
			{ url: 'b', width: 404, height: 720 },
			{ url: 'c', width: 810, height: 1440 },
			{ url: 'd', width: 480, height: 360 }
		];
		expect(pickPortrait(images)?.url).toBe('c');
	});

	it('falls back to landscape images and handles none', () => {
		expect(pickPortrait([{ url: 'd', width: 480, height: 360 }])?.url).toBe('d');
		expect(pickPortrait([])).toBeUndefined();
		expect(pickPortrait(undefined)).toBeUndefined();
	});
});
