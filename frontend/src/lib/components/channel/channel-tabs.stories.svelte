<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ResolvedPathname } from '$app/types';
	import type { PlaylistResult } from '#lib/api/search.js';
	import { mockVideos } from '#lib/mocks/videos.js';
	import ChannelPlaylistGrid from './channel-playlist-grid.svelte';
	import ChannelSort from './channel-sort.svelte';
	import ChannelTabs from './channel-tabs.svelte';
	import ShortGrid from './short-grid.svelte';

	const base = '/@hammerandtongs' as ResolvedPathname;

	const { Story } = defineMeta({
		title: 'Channel/ChannelTabs',
		component: ChannelTabs,
		args: { base, current: 'videos', hasShorts: true, hasLive: true }
	});

	const shorts = mockVideos.map((video, i) => ({
		...video,
		durationSeconds: undefined,
		progress: i === 1 ? 0.4 : undefined
	}));
	const playlists: PlaylistResult[] = mockVideos.slice(0, 6).map((video, i) => ({
		id: `PLmock${i}`,
		title: video.title,
		thumbnailUrl: video.thumbnailUrl,
		videoCount: i === 2 ? undefined : (i + 1) * 7,
		isCourse: i === 1
	}));
</script>

<Story name="All tabs" />

<Story name="Without shorts and live" args={{ hasShorts: false, hasLive: false }} />

<Story name="Search open" args={{ current: 'search', query: 'damascus steel' }} />

<Story name="Sort chips" asChild>
	<ChannelSort {base} tab="videos" current="popular" />
</Story>

<Story name="Shorts grid" asChild>
	<ShortGrid {shorts} label="Shorts" />
</Story>

<Story name="Shorts loading" asChild>
	<ShortGrid loading skeletonCount={6} />
</Story>

<Story name="Playlists grid" asChild>
	<ChannelPlaylistGrid {playlists} label="Playlists" />
</Story>
