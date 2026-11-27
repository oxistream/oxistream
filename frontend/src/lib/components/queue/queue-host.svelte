<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createVideoProgressQuery } from '#lib/api/history.js';
	import { createSettingsQuery, playbackPreferences } from '#lib/api/settings.js';
	import { useManifest, useVideo } from '#lib/api/video.js';
	import { describeProblem, liveProblem } from '#lib/components/watch/errors.js';
	import { readLocalProgress, resumePosition } from '#lib/components/watch/local-progress.js';
	import { watchMedia } from '#lib/components/watch/watch-media.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { session } from '#lib/session.svelte.js';

	// eslint-disable-next-line svelte/no-navigation-without-resolve -- hrefs come resolved
	appPlayer.navigate = (href) => void goto(href);
	appPlayer.watchHref = (id) => resolve(`/watch?v=${id}`);

	// A queued video that plays outside the watch page loads here, as the watch page would.
	const pending = $derived(appPlayer.pending);
	const id = $derived(pending?.id ?? null);
	const video = useVideo(() => id);
	const manifest = useManifest(() => id);
	const settings = createSettingsQuery();
	const savedProgress = createVideoProgressQuery(() => id);
	const settingsKnown = $derived(session.ready && (!session.user || !settings.isPending));
	const startKnown = $derived(session.ready && (!session.user || !savedProgress.isPending));

	$effect(() => {
		if (!pending || !settingsKnown || video.isFetching || manifest.isFetching) return;
		const href = appPlayer.watchHref(pending.id);
		const details = video.data?.id === pending.id ? video.data : undefined;
		const problem = video.isError
			? describeProblem(video.error)
			: details?.is_live
				? liveProblem()
				: manifest.isError
					? describeProblem(manifest.error)
					: null;
		if (problem) {
			appPlayer.load({
				videoId: pending.id,
				title: details?.title ?? pending.title,
				channel: details?.channel ?? pending.channel,
				poster: pending.thumbnailUrl,
				href,
				autoplay: false,
				problem,
				onretry: problem.retry ? retry : undefined
			});
			return;
		}
		if (!details || !manifest.isSuccess || !startKnown) return;
		const saved = session.user ? (savedProgress.data ?? null) : readLocalProgress(pending.id);
		appPlayer.load(
			watchMedia(details, {
				startTime: resumePosition(saved) ?? 0,
				playback: playbackPreferences(settings.data),
				href
			})
		);
	});

	function retry() {
		const media = appPlayer.media;
		if (!media) return;
		appPlayer.pending = {
			id: media.videoId,
			title: media.title,
			channel: media.channel,
			thumbnailUrl: media.poster
		};
		if (video.isError) void video.refetch();
		else void manifest.refetch();
	}
</script>
