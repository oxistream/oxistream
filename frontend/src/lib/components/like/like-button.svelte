<script lang="ts">
	import { page } from '$app/state';
	import { createLikeMutation, createLikeStateQuery } from '#lib/api/likes.js';
	import { authHref } from '#lib/auth/links.js';
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

	const signInHref = $derived(
		session.user ? undefined : authHref('/login', page.url.pathname + page.url.search)
	);
	const liked = $derived(status.data === true);
</script>

<LikeToggle
	{liked}
	{signInHref}
	{count}
	{variant}
	class={className}
	loading={session.user !== null && status.isPending}
	onclick={() => mutation.mutate({ videoId, like: !liked })}
/>
