import { afterEach, describe, expect, it, vi } from 'vitest';
import { createPrefetcher, fragmentsForSeek, isBuffered, type DashEngine } from './prefetch';

const fragments = [0, 5, 10, 15].map((start, i) => ({
	mediaRange: `${1000 + i * 100}-${1099 + i * 100}`,
	presentationStartTime: start,
	duration: 5
}));

describe('fragmentsForSeek', () => {
	it('includes the fragment before when the target is early in its fragment', () => {
		expect(fragmentsForSeek(fragments, 11).map((f) => f.presentationStartTime)).toEqual([5, 10]);
	});

	it('takes a single fragment when the lookup agrees', () => {
		expect(fragmentsForSeek(fragments, 13).map((f) => f.presentationStartTime)).toEqual([10]);
		expect(fragmentsForSeek(fragments, 0).map((f) => f.presentationStartTime)).toEqual([0]);
	});

	it('finds nothing past the end', () => {
		expect(fragmentsForSeek(fragments, 30)).toEqual([]);
	});
});

describe('isBuffered', () => {
	const ranges = (...list: [number, number][]) =>
		({
			length: list.length,
			start: (i: number) => list[i][0],
			end: (i: number) => list[i][1]
		}) as TimeRanges;

	it('needs a second of buffer at the target', () => {
		expect(isBuffered(ranges([0, 30]), 20)).toBe(true);
		expect(isBuffered(ranges([0, 20.5]), 20)).toBe(false);
		expect(isBuffered(ranges([0, 10], [40, 50]), 20)).toBe(false);
	});
});

type Interceptor = Parameters<DashEngine['addRequestInterceptor']>[0];

function fakeEngine() {
	const interceptors: Interceptor[] = [];
	const engine: DashEngine = {
		getCurrentRepresentationForType: (type) =>
			type === 'video' ? { id: '137', segments: fragments } : null,
		addRequestInterceptor: (i) => interceptors.push(i),
		removeRequestInterceptor: (i) => interceptors.splice(interceptors.indexOf(i), 1)
	};
	const request = (range?: string) =>
		interceptors[0]({
			url: '/proxy/stream/abc/137?exp=1&sig=x',
			headers: range ? { Range: range } : {},
			customData: { request: { mediaType: 'video', representation: { id: '137' } } }
		});
	return { engine, interceptors, request };
}

describe('createPrefetcher', () => {
	afterEach(() => vi.unstubAllGlobals());

	it('serves the prefetched bytes and leaves other requests alone', async () => {
		const fetch = vi.fn((_url: string, init: RequestInit) => {
			const [, start, end] = /bytes=(\d+)-(\d+)/.exec(
				(init.headers as Record<string, string>).Range
			)!;
			const body = new Uint8Array(Number(end) - Number(start) + 1);
			return Promise.resolve(new Response(body, { status: 206 }));
		});
		vi.stubGlobal('fetch', fetch);
		const { engine, interceptors, request } = fakeEngine();
		const prefetcher = createPrefetcher(engine);

		await request('bytes=0-99');
		prefetcher.prefetch(13);
		prefetcher.prefetch(13);
		expect(fetch).toHaveBeenCalledTimes(1);
		expect(fetch.mock.calls[0][0]).toBe('/proxy/stream/abc/137?exp=1&sig=x');

		const served = await request('bytes=1200-1299');
		expect(served.url).toMatch(/^blob:/);
		expect(served.headers?.Range).toBeUndefined();

		const again = await request('bytes=1200-1299');
		expect(again.url).toBe('/proxy/stream/abc/137?exp=1&sig=x');
		const other = await request('bytes=1300-1399');
		expect(other.headers?.Range).toBe('bytes=1300-1399');

		prefetcher.destroy();
		expect(interceptors).toHaveLength(0);
	});

	it('falls through when the prefetch comes back short', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(() => Promise.resolve(new Response(new Uint8Array(3), { status: 206 })))
		);
		const { engine, request } = fakeEngine();
		const prefetcher = createPrefetcher(engine);
		await request();
		prefetcher.prefetch(13);
		const served = await request('bytes=1200-1299');
		expect(served.headers?.Range).toBe('bytes=1200-1299');
	});

	it('waits for a stream URL before prefetching', () => {
		const fetch = vi.fn();
		vi.stubGlobal('fetch', fetch);
		createPrefetcher(fakeEngine().engine).prefetch(13);
		expect(fetch).not.toHaveBeenCalled();
	});
});
