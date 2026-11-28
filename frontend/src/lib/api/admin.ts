import {
	createMutation,
	createQuery,
	keepPreviousData,
	useQueryClient
} from '@tanstack/svelte-query';
import { session } from '#lib/session.svelte.js';
import { api } from './client';
import { unwrap } from './mapping';
import type { components } from './schema';

export type AdminUser = components['schemas']['AdminUserEntry'];
export type AdminStatus = components['schemas']['AdminStatus'];
export type AdminUserChange = components['schemas']['AdminUserUpdate'];

export const ADMIN_PAGE_SIZE = 50;

export const adminKeys = {
	all: ['admin'] as const,
	users: (q: string, page: number) => ['admin', 'users', q, page] as const,
	status: ['admin', 'status'] as const
};

export function isAdmin(): boolean {
	return session.account?.role === 'admin';
}

/** One page of users, newest first; idle for non-admins. */
export function createAdminUsersQuery(params: () => { q: string; page: number }) {
	return createQuery(() => {
		const { q, page } = params();
		return {
			queryKey: adminKeys.users(q, page),
			queryFn: async ({ signal }: { signal: AbortSignal }) =>
				unwrap(
					await api.GET('/api/v0/admin/users', {
						params: {
							query: {
								q: q || undefined,
								limit: ADMIN_PAGE_SIZE,
								offset: page * ADMIN_PAGE_SIZE
							}
						},
						signal
					})
				),
			enabled: isAdmin(),
			placeholderData: keepPreviousData
		};
	});
}

export function createAdminStatusQuery() {
	return createQuery(() => ({
		queryKey: adminKeys.status,
		queryFn: async ({ signal }: { signal: AbortSignal }) =>
			unwrap(await api.GET('/api/v0/admin/status', { signal })),
		enabled: isAdmin(),
		refetchInterval: 30_000
	}));
}

export type AdminAction =
	| { kind: 'update'; user: AdminUser; change: AdminUserChange }
	| { kind: 'delete'; user: AdminUser };

export function createAdminUserMutation() {
	const client = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (action: AdminAction) => {
			const path = { id: action.user.id };
			const result =
				action.kind === 'delete'
					? await api.DELETE('/api/v0/admin/users/{id}', { params: { path } })
					: await api.PATCH('/api/v0/admin/users/{id}', {
							params: { path },
							body: action.change
						});
			if (!result.response.ok) unwrap(result);
		},
		onSettled: () => client.invalidateQueries({ queryKey: adminKeys.all })
	}));
}
