import { describe, expect, it } from 'vitest';
import {
	IMPORT_MAX_BYTES,
	exportHref,
	formatLabel,
	importErrorMessage,
	importSubscriptions
} from './subscription-files';

describe('subscription files', () => {
	it('labels detected formats', () => {
		expect(formatLabel('youtube_takeout')).toBe('YouTube Takeout');
		expect(formatLabel('newpipe')).toBe('NewPipe');
		expect(formatLabel('opml')).toBe('OPML');
	});

	it('explains failures by status', () => {
		expect(importErrorMessage(400)).toContain('not a supported');
		expect(importErrorMessage(413)).toContain('5 MB');
		expect(importErrorMessage(500)).toBe(importErrorMessage(502));
	});

	// Uploads are covered end to end; the client needs a browser to resolve relative URLs.
	it('rejects large files before uploading', async () => {
		const outcome = await importSubscriptions(new Blob([new Uint8Array(IMPORT_MAX_BYTES + 1)]));
		expect(outcome).toMatchObject({ ok: false, status: 413 });
	});

	it('links exports', () => {
		expect(exportHref('invidious')).toBe('/api/v0/subscriptions/export?format=invidious');
	});
});
