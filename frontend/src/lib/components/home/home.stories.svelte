<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockContinueWatching, mockVideos } from '#lib/mocks/videos.js';
	import GuestFallback from './guest-fallback.svelte';
	import HomeRow from './home-row.svelte';
	import NoSubscriptions from './no-subscriptions.svelte';
	import { m } from '#lib/paraglide/messages.js';

	const now = new Date('2026-11-26T18:00:00Z');

	const { Story } = defineMeta({
		title: 'Home/HomeRow',
		component: HomeRow,
		parameters: { layout: 'fullscreen' },
		args: { title: 'Latest from your subscriptions', now }
	});
</script>

<Story name="Row" asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<HomeRow
			title={m.home_latest_subscriptions()}
			href="/feed/subscriptions"
			videos={mockVideos}
			{now}
		/>
	</div>
</Story>

<Story name="Without link" asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<HomeRow title={m.home_trending_news()} videos={mockVideos.slice(3)} {now} />
	</div>
</Story>

<Story name="Loading" asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<HomeRow title={m.home_trending_gaming()} loading />
	</div>
</Story>

<Story name="No subscriptions" asChild>
	<div class="p-6 [--page-gutter:1.5rem]">
		<HomeRow title={m.home_latest_subscriptions()}><NoSubscriptions /></HomeRow>
	</div>
</Story>

<Story name="Guest fallback" asChild>
	<div class="flex flex-col gap-8 p-6 [--page-gutter:1.5rem]">
		<GuestFallback recent={mockContinueWatching} {now} />
	</div>
</Story>

<Story name="Guest fallback, nothing watched" asChild>
	<div class="flex flex-col gap-8 p-6 [--page-gutter:1.5rem]">
		<GuestFallback />
	</div>
</Story>
