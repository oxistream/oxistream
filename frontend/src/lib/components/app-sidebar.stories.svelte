<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import {
		mockPendingSubscription,
		mockSubscriptions,
		mockUser,
		mockVideos
	} from '#lib/mocks/videos.js';
	import AppHeader from './app-header.svelte';
	import AppSidebar from './app-sidebar.svelte';
	import VideoGrid from './video/video-grid.svelte';

	const { Story } = defineMeta({
		title: 'Shell/AppSidebar',
		component: AppSidebar,
		parameters: { layout: 'fullscreen' }
	});
</script>

{#snippet shell(open: boolean, user: typeof mockUser | null, channels = mockSubscriptions)}
	<Sidebar.Provider {open}>
		<AppSidebar {user} {channels} />
		<Sidebar.Inset class="overflow-clip md:ring-1 md:ring-sidebar-border">
			<AppHeader />
			<main class="flex-1 px-(--page-gutter) py-6">
				<VideoGrid videos={mockVideos.slice(0, 8)} />
			</main>
		</Sidebar.Inset>
	</Sidebar.Provider>
{/snippet}

<Story name="Signed out" asChild>
	{@render shell(true, null)}
</Story>

<Story name="Signed in" asChild>
	{@render shell(true, mockUser)}
</Story>

<Story name="Signed in with a channel being fetched" asChild>
	{@render shell(true, mockUser, [...mockSubscriptions.slice(0, 3), mockPendingSubscription])}
</Story>

<Story name="Signed in without subscriptions" asChild>
	{@render shell(true, mockUser, [])}
</Story>

<Story name="Collapsed" asChild>
	{@render shell(false, mockUser)}
</Story>

<Story name="Collapsed with a channel being fetched" asChild>
	{@render shell(false, mockUser, [...mockSubscriptions.slice(0, 3), mockPendingSubscription])}
</Story>
