import { api } from '#lib/api/client.js';
import type { components } from './schema';
import { m } from '#lib/paraglide/messages.js';

export type ImportResult = components['schemas']['ImportResult'];
export type ImportFormat = components['schemas']['ImportFormat'];
export type ExportFormat = 'newpipe' | 'invidious';

/** Mirrors the server's limit, so large files fail before the upload. */
export const IMPORT_MAX_BYTES = 5 * 1024 * 1024;

export type ImportOutcome =
	{ ok: true; result: ImportResult } | { ok: false; status: number; message: string };

export function formatLabel(format: ImportFormat): string {
	switch (format) {
		case 'youtube_takeout':
			return m.import_format_youtube_takeout();
		case 'newpipe':
			return 'NewPipe';
		case 'invidious':
			return 'Invidious';
		case 'piped':
			return 'Piped';
		case 'opml':
			return 'OPML';
	}
	return format;
}

/** Explains a failed import; the server's messages are English-only. */
export function importErrorMessage(status: number): string {
	switch (status) {
		case 0:
			return m.import_error_offline();
		case 400:
			return m.import_error_unrecognized();
		case 401:
			return m.import_error_signed_out();
		case 413:
			return m.import_error_too_large();
		default:
			return m.import_error_failed();
	}
}

export async function importSubscriptions(file: Blob): Promise<ImportOutcome> {
	if (file.size > IMPORT_MAX_BYTES) {
		return { ok: false, status: 413, message: importErrorMessage(413) };
	}
	try {
		const { data, response } = await api.POST('/api/v0/subscriptions/import', {
			body: file as unknown as string,
			bodySerializer: (body) => body,
			headers: { 'Content-Type': 'application/octet-stream' }
		});
		if (data) return { ok: true, result: data };
		return { ok: false, status: response.status, message: importErrorMessage(response.status) };
	} catch {
		return { ok: false, status: 0, message: importErrorMessage(0) };
	}
}

export function exportHref(format: ExportFormat): string {
	return `/api/v0/subscriptions/export?format=${format}`;
}
