import { describe, expect, it } from 'vitest';
import { toDiscoveredShort } from './shorts';

describe('shorts discovery', () => {
	it('maps a discovered short to a stub with its portrait thumbnail', () => {
		expect(
			toDiscoveredShort({
				id: 'sHpEqeSByPI',
				channel_id: 'UCX6OQ3DkcsbYNE6H8uQQuVA',
				thumbnails: [
					{ url: '/proxy/image/wide', width: 480, height: 360 },
					{ url: '/proxy/image/tall', width: 720, height: 1280 }
				]
			})
		).toEqual({
			id: 'sHpEqeSByPI',
			title: '',
			channel: { id: 'UCX6OQ3DkcsbYNE6H8uQQuVA', name: '' },
			thumbnailUrl: '/proxy/image/tall'
		});
		expect(toDiscoveredShort({ id: 'x', thumbnails: [] }).channel.id).toBe('');
	});
});
