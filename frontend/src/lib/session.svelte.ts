import { createQuery, type QueryClient } from '@tanstack/svelte-query';
import { api } from '#lib/api/client.js';
import type { components } from './api/schema';
import type { User } from './video';

export type Account = components['schemas']['Account'];
export type AuthCapabilities = components['schemas']['AuthCapabilities'];
export type ApiError = components['schemas']['ErrorBody'];
type InstanceInfo = components['schemas']['InstanceInfo'];

export const sessionKeys = {
	me: ['auth', 'me'] as const,
	instance: ['instance'] as const
};

type QueryState<T> = { readonly data: T | undefined; readonly isPending: boolean };

async function fetchMe(): Promise<Account | null> {
	const { data, response } = await api.GET('/api/v0/auth/me');
	if (response.status === 401) return null;
	if (!data) throw new Error(`GET /auth/me failed with ${response.status}`);
	return data;
}

async function fetchInstance(): Promise<InstanceInfo> {
	const { data, response } = await api.GET('/api/v0/instance');
	if (!data) throw new Error(`GET /instance failed with ${response.status}`);
	return data;
}

class Session {
	#me = $state<QueryState<Account | null>>();
	#instance = $state<QueryState<InstanceInfo>>();
	#client: QueryClient | undefined;

	/** Starts the session queries; call once from the root layout. */
	connect(client: QueryClient) {
		this.#client = client;
		this.#me = createQuery(
			() => ({
				queryKey: sessionKeys.me,
				queryFn: fetchMe,
				staleTime: 5 * 60_000,
				retry: 1
			}),
			() => client
		);
		this.#instance = createQuery(
			() => ({
				queryKey: sessionKeys.instance,
				queryFn: fetchInstance,
				staleTime: Infinity,
				retry: 2
			}),
			() => client
		);
	}

	get account(): Account | null {
		return this.#me?.data ?? null;
	}

	/** `false` until the first session check finished. */
	get ready(): boolean {
		return this.#me !== undefined && !this.#me.isPending;
	}

	/** `null` until the instance information loaded. */
	get capabilities(): AuthCapabilities | null {
		return this.#instance?.data?.auth ?? null;
	}

	get user(): User | null {
		const account = this.account;
		return account ? { id: account.id, name: account.username, username: account.username } : null;
	}

	/** Stores the account returned by login or registration. */
	signedIn(account: Account) {
		this.#client?.setQueryData(sessionKeys.me, account);
	}

	async signOut() {
		await api.POST('/api/v0/auth/logout').catch(() => undefined);
		const client = this.#client;
		if (!client) return;
		// Drop everything that may have been loaded for the previous user.
		client.removeQueries({
			predicate: (query) =>
				query.queryKey[0] !== sessionKeys.me[0] && query.queryKey[0] !== sessionKeys.instance[0]
		});
		client.setQueryData(sessionKeys.me, null);
	}
}

export const session = new Session();
