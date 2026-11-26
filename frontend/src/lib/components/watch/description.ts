import { isHandle, isVideoId, parseYouTubeUrl, watchPath } from '#lib/youtube.js';

export type DescriptionSegment =
	| { type: 'text'; text: string }
	| { type: 'link'; text: string; href: string; internal: boolean }
	| { type: 'timestamp'; text: string; seconds: number };

// URLs, then h:mm:ss / m:ss timestamps that are not part of a longer number or ratio.
const TOKEN = /(https?:\/\/[^\s<>"]+)|(?<![\w:.])((?:\d{1,2}:)?\d{1,2}:[0-5]\d)(?![\w:])/g;

const TRAILING_PUNCTUATION = /[.,;:!?'"*]+$/;

function trimUrl(raw: string): string {
	let url = raw.replace(TRAILING_PUNCTUATION, '');
	// Drop a closing bracket that closes text around the URL rather than one inside it.
	while (url.endsWith(')') && count(url, '(') < count(url, ')')) {
		url = url.slice(0, -1).replace(TRAILING_PUNCTUATION, '');
	}
	return url;
}

function count(text: string, char: string): number {
	return text.split(char).length - 1;
}

/** "1:02:03" → 3723. */
export function parseTimestamp(text: string): number {
	return text
		.split(':')
		.map(Number)
		.reduce((total, part) => total * 60 + part, 0);
}

/**
 * Rewrites a YouTube URL to the matching Oxistream route, so description links stay on this
 * instance. Returns null for anything that has no local equivalent.
 */
export function toLocalPath(href: string): string | null {
	const url = parseYouTubeUrl(href);
	if (!url) return null;
	const [first, second] = url.pathname.split('/').filter(Boolean);
	const query = (key: string) => url.searchParams.get(key);

	if (url.hostname.toLowerCase().endsWith('youtu.be')) {
		return first && isVideoId(first) ? watchPath(first, url.searchParams) : null;
	}
	if (first === 'watch') {
		const id = query('v');
		return id && isVideoId(id) ? watchPath(id, url.searchParams) : null;
	}
	if (
		(first === 'shorts' || first === 'live' || first === 'embed') &&
		second &&
		isVideoId(second)
	) {
		return watchPath(second, url.searchParams);
	}
	if (first === 'channel' && second) return `/channel/${encodeURIComponent(second)}`;
	if (first && isHandle(decodeURIComponent(first))) return `/${first}`;
	if (first === 'playlist' && query('list')) {
		return `/playlist?${new URLSearchParams({ list: query('list')! })}`;
	}
	if (first === 'results' && query('search_query')) {
		return `/results?${new URLSearchParams({ search_query: query('search_query')! })}`;
	}
	return null;
}

/**
 * Splits a plain-text description into text, links and timestamps. Timestamps past `duration`
 * (when known) stay plain text, since they cannot point into this video.
 */
export function parseDescription(text: string, duration?: number | null): DescriptionSegment[] {
	const segments: DescriptionSegment[] = [];
	let last = 0;
	const push = (segment: DescriptionSegment) => {
		const previous = segments[segments.length - 1];
		if (segment.type === 'text' && previous?.type === 'text') previous.text += segment.text;
		else segments.push(segment);
	};

	for (const match of text.matchAll(TOKEN)) {
		const start = match.index;
		const [raw, rawUrl, stamp] = match;
		let token: DescriptionSegment | null = null;
		let length = raw.length;

		if (rawUrl) {
			const url = trimUrl(rawUrl);
			length = url.length;
			const local = toLocalPath(url);
			token = { type: 'link', text: url, href: local ?? url, internal: local !== null };
		} else if (stamp) {
			const seconds = parseTimestamp(stamp);
			if (duration == null || seconds <= duration) {
				token = { type: 'timestamp', text: stamp, seconds };
			}
		}

		if (!token) continue;
		if (start > last) push({ type: 'text', text: text.slice(last, start) });
		push(token);
		last = start + length;
	}
	if (last < text.length) push({ type: 'text', text: text.slice(last) });
	return segments;
}
