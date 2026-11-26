<script lang="ts">
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import type { Snippet } from 'svelte';

	const { children }: { children: Snippet } = $props();

	// Storybook has no backend, so queries a story triggers must fail fast instead of retrying.
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
</script>

<QueryClientProvider {client}>
	{@render children()}
</QueryClientProvider>
