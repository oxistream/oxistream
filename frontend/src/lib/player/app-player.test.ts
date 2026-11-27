import { describe, expect, it, vi } from 'vitest';
import { AppPlayer, type PlayerMedia } from './app-player.svelte';

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
});
