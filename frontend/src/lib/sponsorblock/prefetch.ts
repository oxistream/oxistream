/** A media fragment as dash.js lists it for a `SegmentBase` representation (from the sidx or WebM cues). */
export type Fragment = { mediaRange: string; presentationStartTime: number; duration: number };

/**
 * The fragments dash.js requests when seeking to `time`: the one its segment lookup picks, which
 * tolerates half a fragment and may be the one before, and the one containing `time`.
 */
export function fragmentsForSeek(fragments: Fragment[], time: number): Fragment[] {
	const picked = fragments.find(
		(f) =>
			time + f.duration / 2 >= f.presentationStartTime &&
			time - f.duration / 2 < f.presentationStartTime + f.duration
	);
	const containing = fragments.find(
		(f) => time >= f.presentationStartTime && time < f.presentationStartTime + f.duration
	);
	return [...new Set([picked, containing])].filter((f) => f !== undefined);
}

/** Whether playback can resume at `time` from the buffer alone. */
export function isBuffered(buffered: TimeRanges, time: number): boolean {
	for (let i = 0; i < buffered.length; i++) {
		if (buffered.start(i) <= time && buffered.end(i) > time + 1) return true;
	}
	return false;
}

type MediaType = 'video' | 'audio';

type Request = {
	url: string;
	headers?: Record<string, string>;
	customData?: { request?: { mediaType?: string; representation?: { id: string } | null } };
};

/** The parts of the dash.js `MediaPlayer` the prefetcher uses. */
export type DashEngine = {
	getCurrentRepresentationForType(
		type: MediaType
	): { id: string; segments?: Fragment[] | null } | null;
	addRequestInterceptor(interceptor: (request: Request) => Promise<Request>): void;
	removeRequestInterceptor(interceptor: (request: Request) => Promise<Request>): void;
};

type Entry = { data: Promise<Blob | null>; created: number };

const MAX_ENTRIES = 6;
const TTL_MS = 60_000;
const WAIT_MS = 5_000;

/**
 * Loads the fragments at a skip target ahead of time and hands them to dash.js when it asks for
 * exactly those bytes. Anything else, or a failed prefetch, goes to the network unchanged.
 */
export function createPrefetcher(engine: DashEngine) {
	const urls = new Map<string, string>();
	const entries = new Map<string, Entry>();
	const requested = new Set<string>();

	const expired = (entry: Entry) => Date.now() - entry.created > TTL_MS;

	const intercept = async (request: Request): Promise<Request> => {
		const fragment = request.customData?.request;
		if (fragment?.mediaType && fragment.representation) {
			urls.set(`${fragment.mediaType}:${fragment.representation.id}`, request.url);
		}
		const range = request.headers?.Range;
		const key = `${request.url}|${range}`;
		const entry = range ? entries.get(key) : undefined;
		if (!entry) return request;
		entries.delete(key);
		if (expired(entry)) return request;
		const blob = await Promise.race([
			entry.data,
			new Promise<null>((resolve) => setTimeout(() => resolve(null), WAIT_MS))
		]);
		if (!blob) return request;
		const url = URL.createObjectURL(blob);
		setTimeout(() => URL.revokeObjectURL(url), TTL_MS);
		// dash.js sets the header again on a retry, which then goes to the network.
		delete request.headers!.Range;
		request.url = url;
		return request;
	};
	engine.addRequestInterceptor(intercept);

	const load = (url: string, range: string, size: number) =>
		fetch(url, { headers: { Range: range }, credentials: 'omit' })
			.then((response) => (response.ok ? response.blob() : null))
			.then((blob) => (blob?.size === size ? blob : null))
			.catch(() => null);

	return {
		/** Loads what dash.js will request after seeking to `time`, once per target. */
		prefetch(time: number) {
			for (const [key, entry] of entries) if (expired(entry)) entries.delete(key);
			for (const type of ['video', 'audio'] as const) {
				const representation = engine.getCurrentRepresentationForType(type);
				const url = representation && urls.get(`${type}:${representation.id}`);
				const once = `${type}:${representation?.id}:${time}`;
				if (!url || !representation.segments?.length || requested.has(once)) continue;
				requested.add(once);
				for (const fragment of fragmentsForSeek(representation.segments, time)) {
					const [start, end] = fragment.mediaRange.split('-').map(Number);
					const range = `bytes=${fragment.mediaRange}`;
					const key = `${url}|${range}`;
					if (entries.has(key) || !(end >= start)) continue;
					while (entries.size >= MAX_ENTRIES) entries.delete(entries.keys().next().value!);
					entries.set(key, { data: load(url, range, end - start + 1), created: Date.now() });
				}
			}
		},
		destroy() {
			engine.removeRequestInterceptor(intercept);
			entries.clear();
		}
	};
}

export type Prefetcher = ReturnType<typeof createPrefetcher>;
