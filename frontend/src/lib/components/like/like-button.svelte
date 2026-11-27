<script lang="ts">
	import { createLikeMutation, createLikeStateQuery } from '#lib/api/likes.js';
	import { session } from '#lib/session.svelte.js';
	import LikeToggle from './like-toggle.svelte';

	let {
		videoId,
		count,
		variant = 'default',
		class: className
	}: {
		videoId: string;
		count?: number | null;
		variant?: 'default' | 'overlay';
		class?: string;
	} = $props();

	const status = createLikeStateQuery(() => videoId);
	const mutation = createLikeMutation();

	const liked = $derived(status.data === true);
</script>

{#if session.user}
	<LikeToggle
		{liked}
		{count}
		{variant}
		class={className}
		loading={status.isPending}
		onclick={() => mutation.mutate({ videoId, like: !liked })}
	/>
{/if}
