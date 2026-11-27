<script lang="ts">
	import BookmarkCheckIcon from '@lucide/svelte/icons/bookmark-check';
	import BookmarkPlusIcon from '@lucide/svelte/icons/bookmark-plus';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		following = false,
		loading = false,
		onclick,
		class: className
	}: {
		following?: boolean;
		/** The follow state is not known yet. */
		loading?: boolean;
		onclick?: () => void;
		class?: string;
	} = $props();
</script>

<Button
	variant={following ? 'secondary' : 'outline'}
	class={cn(loading && 'cursor-wait', className)}
	aria-pressed={following}
	aria-busy={loading || undefined}
	disabled={loading}
	data-testid="follow-button"
	{onclick}
>
	{#if following}
		<BookmarkCheckIcon class="text-primary" />
		{m.playlist_following()}
	{:else}
		<BookmarkPlusIcon />
		{m.playlist_follow()}
	{/if}
</Button>
