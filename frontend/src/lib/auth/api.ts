import { api } from '#lib/api/client.js';
import type { Account, ApiError, AuthCapabilities } from '#lib/session.svelte.js';
import { m } from '#lib/paraglide/messages.js';
import { redirectQuery } from './redirect';

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

/** Starts single sign-on; a full page navigation, as the provider redirects back to the API. */
export function oidcLoginHref(target: string): string {
	return `/api/v0/auth/oidc/login${redirectQuery(target)}`;
}

/** The single sign-on button returning to `target`, if the instance offers it. */
export function ssoButton(
	capabilities: AuthCapabilities | null,
	target: string
): { label: string; href: string } | undefined {
	if (!capabilities?.oidc_enabled) return undefined;
	return {
		label: m.auth_oidc_sign_in({ name: capabilities.oidc_name || 'SSO' }),
		href: oidcLoginHref(target)
	};
}

/** Message for the `?error=` code the single sign-on callback redirects to `/login` with. */
export function oidcErrorMessage(code: string | null): string | undefined {
	switch (code) {
		case null:
			return undefined;
		case 'oidc_disabled':
			return m.auth_oidc_error_disabled();
		case 'oidc_unavailable':
			return m.auth_oidc_error_unavailable();
		case 'oidc_denied':
			return m.auth_oidc_error_denied();
		case 'account_disabled':
			return m.auth_error_account_disabled();
		default:
			return m.auth_oidc_error_failed();
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
