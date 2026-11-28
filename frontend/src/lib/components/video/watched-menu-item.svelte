<script lang="ts">
	import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
	import CircleDashedIcon from '@lucide/svelte/icons/circle-dashed';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import { createVideoProgressQuery, setWatched } from '#lib/api/history.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let { video }: { video: { id: string; durationSeconds?: number } } = $props();

	const client = useQueryClient();
	// Rendered only while the menu is open, so this asks once per opening at most.
	const progress = createVideoProgressQuery(() => video.id);
	const watched = $derived(progress.data?.finished === true);

	function select() {
		setWatched(client, video, !watched).catch(() => toast.error(m.video_action_failed()));
	}
</script>

<DropdownMenu.Item
	onSelect={select}
	disabled={progress.isPending}
	data-testid="mark-watched"
	data-watched={watched}
>
	{#if watched}
		<CircleDashedIcon />
		{m.video_mark_unwatched()}
	{:else}
		<CircleCheckIcon />
		{m.video_mark_watched()}
	{/if}
</DropdownMenu.Item>
