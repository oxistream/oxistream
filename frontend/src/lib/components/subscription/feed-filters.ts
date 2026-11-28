import type { SubscribedChannel } from '#lib/api/subscription-list.js';

/** Whether the channel's name or handle contains the search term, ignoring case and accents. */
export function matchesChannel(
	channel: Pick<SubscribedChannel, 'id' | 'name' | 'handle'>,
	search: string
): boolean {
	const term = fold(search.trim());
	if (!term) return true;
	return [channel.name ?? channel.id, channel.handle ?? ''].some((text) =>
		fold(text).includes(term)
	);
}

function fold(text: string): string {
	return text
		.normalize('NFD')
		.replace(/\p{Diacritic}/gu, '')
		.toLowerCase();
}
