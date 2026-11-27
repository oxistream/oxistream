import { describe, expect, it, vi } from 'vitest';
import { AppPlayer, type PlayerMedia } from './app-player.svelte';
import { UpNextQueue } from './queue.svelte';

const media = (videoId: string): PlayerMedia => ({
	videoId,
	title: videoId,
	href: `/watch?v=${videoId}`
});

describe('AppPlayer', () => {
	it('loads a new video over the current one', () => {
		const player = new AppPlayer();
		expect(player.load(media('a'))).toBe(true);
		const first = player.generation;
		player.load(media('b'));
		expect(player.media?.videoId).toBe('b');
		expect(player.generation).toBe(first + 1);
	});

	it('is inline only while a slot holds it', () => {
		const player = new AppPlayer();
		player.load(media('a'));
		const node = {} as HTMLElement;
		const detach = player.slot(() => ({ theater: true }))(node);
		expect(player.inline).toBe(true);
		expect(player.slotOptions().theater).toBe(true);
		detach?.();
		expect(player.inline).toBe(false);
		expect(player.slotOptions()).toEqual({});
	});

	it('keeps the newer slot when an older one goes away', () => {
		const player = new AppPlayer();
		const old = {} as HTMLElement;
		const next = {} as HTMLElement;
		const detachOld = player.slot()(old);
		player.slot()(next);
		detachOld?.();
		expect(player.slotElement).toBe(next);
	});

	it('refuses other loads while claimed and lets the holder decide on close', () => {
		const player = new AppPlayer();
		const onclose = vi.fn();
		const claim = player.take('party', { onclose })!;
		expect(player.take('other')).toBeNull();
		claim.load(media('party-video'));
		expect(player.load(media('page-video'))).toBe(false);
		expect(player.media?.videoId).toBe('party-video');

		player.close();
		expect(onclose).toHaveBeenCalledOnce();
		expect(player.media).not.toBeNull();

		claim.release();
		expect(player.claim).toBeNull();
		claim.load(media('stale'));
		expect(player.media?.videoId).toBe('party-video');
		player.close();
		expect(player.media).toBeNull();
	});

	it('plays the queue in order, then where the page continues', () => {
		const player = new AppPlayer(new UpNextQueue(null));
		const navigate = vi.fn();
		player.navigate = navigate;
		player.load(media('a'));
		player.slot(() => ({ next: '/watch?v=p2&list=PL' }))({} as HTMLElement);
		player.enqueue({ id: 'q1', title: 'q1' });
		player.enqueue({ id: 'q2', title: 'q2' });
		player.enqueue({ id: 'q0', title: 'q0' }, { next: true });
		expect(player.queue.entries.map((e) => e.id)).toEqual(['q0', 'q1', 'q2']);
		expect(player.hasNext).toBe(true);

		expect(player.next()).toBe(true);
		expect(navigate).toHaveBeenLastCalledWith('/watch?v=q0');
		// The watch page of q0 has no playlist, so the queue goes on and then returns to it.
		player.slot(() => ({}))({} as HTMLElement);
		player.load(media('q0'));
		player.next();
		player.load(media('q1'));
		player.next();
		player.load(media('q2'));
		expect(player.next()).toBe(true);
		expect(navigate).toHaveBeenLastCalledWith('/watch?v=p2&list=PL');
		expect(player.next()).toBe(false);
	});

	it('forgets the playlist once another video is opened', () => {
		const player = new AppPlayer(new UpNextQueue(null));
		const navigate = vi.fn();
		player.navigate = navigate;
		player.load(media('a'));
		player.slot(() => ({ next: '/watch?v=p2&list=PL' }))({} as HTMLElement);
		player.enqueue({ id: 'q1', title: 'q1' });
		player.next();
		player.slot(() => ({}))({} as HTMLElement);
		player.load(media('other'));
		expect(player.hasNext).toBe(false);
		expect(player.next()).toBe(false);
	});

	it('loads queued videos outside the watch page through the host', () => {
		const player = new AppPlayer(new UpNextQueue(null));
		player.enqueue({ id: 'q1', title: 'q1' });
		expect(player.pending?.id).toBe('q1');
		player.load(media('q1'));
		expect(player.pending).toBeNull();
		player.enqueue({ id: 'q2', title: 'q2' });
		// Something plays, so it waits its turn.
		expect(player.pending).toBeNull();
		expect(player.next()).toBe(true);
		expect(player.pending?.id).toBe('q2');
		player.load(media('q2'));
		// It played to the end with nothing queued, so the next one starts at once.
		player.ended = true;
		player.enqueue({ id: 'q3', title: 'q3' });
		expect(player.pending?.id).toBe('q3');
	});

	it('goes back to the previous video and queues the current one again', () => {
		const player = new AppPlayer(new UpNextQueue(null));
		player.load(media('a'));
		player.load(media('b'));
		expect(player.previous()).toBe(true);
		expect(player.pending?.id).toBe('a');
		expect(player.queue.entries.map((e) => e.id)).toEqual(['b']);
		player.load(media('a'));
		// Nothing before `a`: it starts over.
		const seek = vi.fn();
		player.controls = { seek, position: () => 0 } as unknown as NonNullable<AppPlayer['controls']>;
		expect(player.previous()).toBe(true);
		expect(seek).toHaveBeenCalledWith(0);
		expect(player.pending).toBeNull();
	});

	it('leaves next and previous to the holder of a claim', () => {
		const player = new AppPlayer(new UpNextQueue(null));
		player.enqueue({ id: 'q1', title: 'q1' });
		player.load(media('q1'));
		player.enqueue({ id: 'q2', title: 'q2' });
		const claim = player.take('party')!;
		expect(player.hasNext).toBe(false);
		expect(player.next()).toBe(false);
		expect(player.queue.length).toBe(1);
		claim.release();
		const onnext = vi.fn();
		player.take('party', { onnext });
		expect(player.next()).toBe(true);
		expect(onnext).toHaveBeenCalledOnce();
	});
});
