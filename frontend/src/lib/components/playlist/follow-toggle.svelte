<script lang="ts">
	import BookmarkIcon from '@lucide/svelte/icons/bookmark';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import IconAction from './icon-action.svelte';

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

<IconAction
	label={m.playlist_follow()}
	tooltip={following ? m.playlist_following() : m.playlist_follow()}
	class={cn(loading && 'cursor-wait', className)}
	aria-pressed={following}
	aria-busy={loading || undefined}
	disabled={loading}
	data-testid="follow-button"
	{onclick}
>
	<BookmarkIcon
		class={cn(
			'transition-colors duration-(--motion-in) ease-ui',
			following && 'fill-primary text-primary'
		)}
	/>
</IconAction>
