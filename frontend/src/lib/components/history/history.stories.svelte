<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { groupByDay, type HistoryVideo } from '#lib/api/history-list.js';
	import { mockVideos } from '#lib/mocks/videos.js';
	import ClearHistoryDialog from './clear-history-dialog.svelte';
	import HistoryList from './history-list.svelte';
	import HistoryPausedNotice from './history-paused-notice.svelte';

	const now = new Date('2026-11-26T18:00:00Z');
	const HOUR = 3600 * 1000;

	const watched: HistoryVideo[] = mockVideos.slice(0, 7).map((video, i) => ({
		...video,
		progress: video.progress ?? (i % 3 === 0 ? 1 : 0.15 * (i + 1)),
		watchedAt: new Date(now.getTime() - i * 14 * HOUR).toISOString()
	}));

	const { Story } = defineMeta({
		title: 'History/HistoryList',
		component: HistoryList,
		args: {
			now,
			days: groupByDay(watched, { now, locale: 'en', today: 'Today', yesterday: 'Yesterday' })
		}
	});
</script>

<script lang="ts">
	let removed = $state<string[]>([]);
	const remaining = $derived(
		groupByDay(
			watched.filter((v) => !removed.includes(v.id)),
			{ now, locale: 'en', today: 'Today', yesterday: 'Yesterday' }
		)
	);
</script>

<Story name="Default" />

<Story name="Loading" args={{ loading: true }} />

<Story name="Removable" asChild>
	<HistoryList {now} days={remaining} onremove={(id) => (removed = [...removed, id])} />
</Story>

<Story name="Paused notice" asChild>
	<HistoryPausedNotice class="max-w-3xl" />
</Story>

<Story name="Clear dialog" asChild>
	<ClearHistoryDialog onconfirm={() => (removed = watched.map((v) => v.id))} />
</Story>
