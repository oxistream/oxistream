<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import { resolve } from '$app/paths';
	import { createContinueWatchingQuery } from '#lib/api/history.js';
	import PageHeader from '#lib/components/page-header.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import VideoGrid from '#lib/components/video/video-grid.svelte';
	import { mockRecent } from '#lib/mocks/videos.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';

	const continueWatching = createContinueWatchingQuery();
	const showContinue = $derived(
		session.user !== null && (continueWatching.isPending || !!continueWatching.data?.length)
	);
</script>

<div class="flex flex-col gap-8 px-(--page-gutter) py-6">
	<PageHeader title={m.nav_home()} documentTitle={pageTitle()} class="sr-only" />

	{#if showContinue}
		<section aria-labelledby="continue-heading" class="flex flex-col gap-3">
			<div class="flex items-center justify-between gap-4">
				<h2 id="continue-heading" class="text-base font-semibold tracking-tight">
					{m.home_continue_watching()}
				</h2>
				<Button href={resolve('/feed/history')} variant="ghost" size="sm">
					{m.history_view_all()}
					<ArrowRightIcon />
				</Button>
			</div>
			<VideoGrid
				videos={continueWatching.data ?? []}
				loading={continueWatching.isPending}
				skeletonCount={4}
				layout="row"
				label={m.home_continue_watching()}
			/>
		</section>
	{/if}

	<section aria-labelledby="recent-heading" class="flex flex-col gap-3">
		<h2 id="recent-heading" class="text-base font-semibold tracking-tight">{m.home_recent()}</h2>
		<VideoGrid videos={mockRecent} />
	</section>
</div>
