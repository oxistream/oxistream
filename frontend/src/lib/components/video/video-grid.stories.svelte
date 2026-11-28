<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import ListVideoIcon from '@lucide/svelte/icons/list-video';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import { mockContinueWatching, mockVideos } from '#lib/mocks/videos.js';
	import { m } from '#lib/paraglide/messages.js';
	import VideoGrid from './video-grid.svelte';

	const { Story } = defineMeta({
		title: 'Video/VideoGrid',
		component: VideoGrid,
		parameters: { layout: 'fullscreen' }
	});
</script>

<Story name="Grid" asChild>
	<div class="p-6"><VideoGrid videos={mockVideos} /></div>
</Story>

<Story name="Row" asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<VideoGrid videos={mockContinueWatching} layout="row" />
	</div>
</Story>

<Story name="Loading" asChild>
	<div class="p-6"><VideoGrid loading skeletonCount={8} /></div>
</Story>

<Story name="Empty" asChild>
	<div class="p-6">
		<EmptyState icon={ListVideoIcon} title={m.empty_playlists_title()} />
	</div>
</Story>

<Story name="Error" asChild>
	<div class="p-6"><ErrorState onretry={() => {}} /></div>
</Story>
