<script lang="ts">
	import LinkIcon from '@lucide/svelte/icons/link';
	import LockIcon from '@lucide/svelte/icons/lock';
	import type { PlaylistVisibility } from '#lib/api/playlists.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { visibility, class: className }: { visibility: PlaylistVisibility; class?: string } =
		$props();

	const Icon = $derived(visibility === 'unlisted' ? LinkIcon : LockIcon);
</script>

<span class={cn('inline-flex items-center gap-1', className)}>
	<Icon class="size-3.5 shrink-0" aria-hidden="true" />
	{visibility === 'unlisted' ? m.playlist_visibility_unlisted() : m.playlist_visibility_private()}
</span>
