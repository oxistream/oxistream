import jsQR from 'jsqr';
import { describe, expect, it } from 'vitest';
import { qrSvg } from './qr.js';

function rasterize(size: number, path: string, scale: number) {
	const px = size * scale;
	const pixels = new Uint8ClampedArray(px * px * 4).fill(255);
	for (const [, x, y, w] of path.matchAll(/M(\d+) (\d+)h(\d+)/g)) {
		for (let row = +y * scale; row < (+y + 1) * scale; row++) {
			const offset = row * px;
			for (let col = +x * scale; col < (+x + +w) * scale; col++) {
				pixels.fill(0, (offset + col) * 4, (offset + col) * 4 + 3);
			}
		}
	}
	return { pixels, px };
}

describe('qrSvg', () => {
	it('encodes the invite link', () => {
		const url = 'https://tube.example/party/Qm9vbS1vbmUtMTIzNDU2Nw';
		const { size, path } = qrSvg(url);
		const { pixels, px } = rasterize(size, path, 4);
		expect(jsQR(pixels, px, px)?.data).toBe(url);
	});

	it('keeps a four-module quiet zone', () => {
		const { size, path } = qrSvg('https://tube.example/party/abc');
		for (const [, x, y, w] of path.matchAll(/M(\d+) (\d+)h(\d+)/g)) {
			expect(+x).toBeGreaterThanOrEqual(4);
			expect(+y).toBeGreaterThanOrEqual(4);
			expect(+x + +w).toBeLessThanOrEqual(size - 4);
			expect(+y).toBeLessThan(size - 4);
		}
	});
});
