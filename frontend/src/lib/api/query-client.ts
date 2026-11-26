import { QueryClient } from '@tanstack/svelte-query';

export function createQueryClient(): QueryClient {
	return new QueryClient({
		defaultOptions: {
			queries: {
				staleTime: 60_000,
				refetchOnWindowFocus: false
			}
		}
	});
}
