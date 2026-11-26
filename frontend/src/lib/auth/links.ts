import { resolve } from '$app/paths';
import type { ResolvedPathname } from '$app/types';
import { redirectQuery, safeRedirect } from './redirect';

/** Link to the sign-in or registration page that returns to `target` afterwards. */
export function authHref(route: '/login' | '/register', target: string): ResolvedPathname {
	return `${resolve(route)}${redirectQuery(target)}` as ResolvedPathname;
}

/** The validated `?redirect=` target of the current auth page. */
export function redirectTarget(url: {
	searchParams: { get(name: string): string | null };
}): ResolvedPathname {
	return safeRedirect(url.searchParams.get('redirect')) as ResolvedPathname;
}
