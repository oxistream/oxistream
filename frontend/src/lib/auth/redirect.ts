const BASE = 'http://oxistream.invalid';

/** A same-origin path to continue to after signing in; anything else falls back to home. */
export function safeRedirect(target: string | null | undefined): string {
	if (!target || !target.startsWith('/') || target.startsWith('//') || target.startsWith('/\\')) {
		return '/';
	}
	try {
		const url = new URL(target, BASE);
		if (url.origin !== BASE || url.pathname === '/login' || url.pathname === '/register') {
			return '/';
		}
		return url.pathname + url.search + url.hash;
	} catch {
		return '/';
	}
}

/** Query string for the sign-in page that returns to `path` afterwards. */
export function redirectQuery(path: string): string {
	const target = safeRedirect(path);
	return target === '/' ? '' : `?${new URLSearchParams({ redirect: target })}`;
}
