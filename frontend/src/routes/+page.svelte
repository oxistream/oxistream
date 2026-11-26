<script lang="ts">
	import PageHeader from '#lib/components/page-header.svelte';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { mockContinueWatching, mockRecent } from '#lib/mocks/videos.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
</script>

<div class="flex flex-col gap-8 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_home()} documentTitle={pageTitle()} class="sr-only" />

	<section aria-labelledby="continue-heading" class="flex flex-col gap-3">
		<h2 id="continue-heading" class="text-base font-semibold tracking-tight">
			{m.home_continue_watching()}
		</h2>
		<VideoGrid videos={mockContinueWatching} layout="row" />
	</section>

	<section aria-labelledby="recent-heading" class="flex flex-col gap-3">
		<h2 id="recent-heading" class="text-base font-semibold tracking-tight">{m.home_recent()}</h2>
		<VideoGrid videos={mockRecent} />
	</section>
</div>
