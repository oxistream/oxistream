import { encode } from 'uqr';

export interface QrSvg {
	size: number;
	path: string;
}

/** Encodes `text` as a QR code with a 4-module quiet zone, as one SVG path of dark modules. */
export function qrSvg(text: string): QrSvg {
	const { size, data } = encode(text, { ecc: 'M', border: 4 });
	let path = '';
	data.forEach((row, y) => {
		let x = 0;
		while (x < size) {
			if (!row[x]) {
				x++;
				continue;
			}
			const start = x;
			while (x < size && row[x]) x++;
			path += `M${start} ${y}h${x - start}v1h${start - x}z`;
		}
	});
	return { size, path };
}
