export type DescriptionSegment =
	| { type: 'text'; text: string }
	| { type: 'link'; text: string; href: string; internal: boolean }
	| { type: 'timestamp'; text: string; seconds: number };

const YOUTUBE_HOSTS = new Set([
	'youtube.com',
	'www.youtube.com',
	'm.youtube.com',
	'music.youtube.com',
	'youtu.be',
	'www.youtu.be'
]);

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

const VIDEO_ID = /^[\w-]{11}$/;

/**
 * Rewrites a YouTube URL to the matching Oxistream route, so description links stay on this
 * instance. Returns null for anything that has no local equivalent.
 */
export function toLocalPath(href: string): string | null {
	let url: URL;
	try {
		url = new URL(href);
	} catch {
		return null;
	}
	if (!YOUTUBE_HOSTS.has(url.hostname)) return null;

	const time = url.searchParams.get('t');
	const withTime = (path: string) => (time ? `${path}&t=${encodeURIComponent(time)}` : path);

	if (url.hostname.endsWith('youtu.be')) {
		const id = url.pathname.slice(1);
		return VIDEO_ID.test(id) ? withTime(`/watch?v=${id}`) : null;
	}

	const [first, second] = url.pathname.split('/').filter(Boolean);
	if (first === 'watch') {
		const id = url.searchParams.get('v');
		return id && VIDEO_ID.test(id) ? withTime(`/watch?v=${id}`) : null;
	}
	if ((first === 'shorts' || first === 'live') && second && VIDEO_ID.test(second)) {
		return `/watch?v=${second}`;
	}
	if (first === 'channel' && second) return `/channel/${second}`;
	if (first?.startsWith('@')) return `/${first}`;
	if (first === 'playlist' && url.searchParams.has('list')) {
		return `/playlist?list=${encodeURIComponent(url.searchParams.get('list')!)}`;
	}
	if (first === 'results' && url.searchParams.has('search_query')) {
		return `/results?search_query=${encodeURIComponent(url.searchParams.get('search_query')!)}`;
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
