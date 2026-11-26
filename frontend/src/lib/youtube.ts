const VIDEO_ID = /^[A-Za-z0-9_-]{10}[AEIMQUYcgkosw048]$/;
// YouTube handles: 3-30 letters, digits, dots, dashes, underscores; localized letters are allowed.
const HANDLE = /^@[\p{L}\p{N}._-]{3,30}$/u;
const YOUTUBE_HOSTS = ['youtube.com', 'youtu.be', 'youtube-nocookie.com'];

/**
 * An 11-character YouTube video id. The last character only carries 4 bits, which keeps words
 * such as "information" from being mistaken for an id.
 */
export function isVideoId(value: string): boolean {
	return VIDEO_ID.test(value);
}

/** A channel handle with its leading "@", as in "/@handle". */
export function isHandle(value: string): boolean {
	return HANDLE.test(value);
}

/** Parses a pasted YouTube link, with or without scheme. Returns null for anything else. */
export function parseYouTubeUrl(input: string): URL | null {
	const value = input.trim();
	if (!value || /\s/.test(value)) return null;
	const withScheme = /^https?:\/\//i.test(value) ? value : `https://${value}`;
	let url: URL;
	try {
		url = new URL(withScheme);
	} catch {
		return null;
	}
	if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
	const host = url.hostname.toLowerCase();
	const known = YOUTUBE_HOSTS.some((base) => host === base || host.endsWith(`.${base}`));
	if (!known) return null;
	// A bare host is a search for the word "youtube.com", not a link to resolve.
	if (url.pathname === '/' && !url.search) return null;
	return url;
}

export type ResolvedTarget = {
	kind: 'video' | 'channel' | 'playlist';
	id: string;
	start_seconds?: number | null;
};

/** The in-app path for a resolved YouTube link, mirroring YouTube's own routes. */
export function resolvedPath(target: ResolvedTarget): string {
	switch (target.kind) {
		case 'video': {
			const params = new URLSearchParams({ v: target.id });
			if (target.start_seconds) params.set('t', String(target.start_seconds));
			return `/watch?${params.toString()}`;
		}
		case 'channel':
			return `/channel/${encodeURIComponent(target.id)}`;
		case 'playlist':
			return `/playlist?${new URLSearchParams({ list: target.id }).toString()}`;
	}
}

/** `/watch?v=` for a bare video id, keeping a `t` start offset from the original link. */
export function watchPath(id: string, search?: URLSearchParams): string {
	const params = new URLSearchParams({ v: id });
	const t = search?.get('t') ?? search?.get('start');
	if (t) params.set('t', t);
	return `/watch?${params.toString()}`;
}
