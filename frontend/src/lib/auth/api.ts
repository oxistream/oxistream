import { api } from '#lib/api/client.js';
import type { Account, ApiError } from '#lib/session.svelte.js';
import { m } from '#lib/paraglide/messages.js';

export type AuthResult =
	| { ok: true; account: Account }
	/** `error` is `null` when the server could not be reached or sent no error body. */
	| { ok: false; status: number; error: ApiError | null };

type Credentials = { username: string; password: string };

export async function login(body: Credentials): Promise<AuthResult> {
	try {
		const { data, error, response } = await api.POST('/api/v0/auth/login', { body });
		return data ? { ok: true, account: data } : { ok: false, status: response.status, error };
	} catch {
		return { ok: false, status: 0, error: null };
	}
}

export async function register(body: Credentials): Promise<AuthResult> {
	try {
		const { data, error, response } = await api.POST('/api/v0/auth/register', { body });
		return data ? { ok: true, account: data } : { ok: false, status: response.status, error };
	} catch {
		return { ok: false, status: 0, error: null };
	}
}

/** Form-level message for a failed request; field errors are shown on the fields instead. */
export function failureMessage(status: number, error: ApiError | null): string {
	if (status === 429) return m.auth_error_too_many_attempts();
	switch (error?.code) {
		case 'invalid_credentials':
			return m.auth_error_invalid_credentials();
		case 'account_disabled':
			return m.auth_error_account_disabled();
		case 'too_many_requests':
			return m.auth_error_too_many_attempts();
		case 'registration_disabled':
			return m.auth_error_registration_disabled();
		case 'local_auth_disabled':
			return m.auth_local_disabled();
		default:
			return m.auth_error_generic();
	}
}
