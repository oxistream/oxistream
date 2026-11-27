import { m } from '#lib/paraglide/messages.js';
import type { RoomErrorCode } from '#lib/together/connection.svelte.js';

const messages: Partial<Record<RoomErrorCode, () => string>> = {
	queue_full: m.party_error_queue_full,
	video_unavailable: m.party_error_video,
	rate_limited: m.party_error_rate
};

/** What the party server said about something this tab sent. */
export function addErrorMessage(code: RoomErrorCode): string {
	return (messages[code] ?? m.party_error_generic)();
}
