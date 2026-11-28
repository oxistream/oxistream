import { m } from '#lib/paraglide/messages.js';
import type { RoomReaction } from './connection.svelte';

export const REACTIONS: { id: RoomReaction; emoji: string; label: () => string }[] = [
	{ id: 'heart', emoji: '❤️', label: m.party_reaction_heart },
	{ id: 'laugh', emoji: '😂', label: m.party_reaction_laugh },
	{ id: 'wow', emoji: '😮', label: m.party_reaction_wow },
	{ id: 'sad', emoji: '😢', label: m.party_reaction_sad },
	{ id: 'clap', emoji: '👏', label: m.party_reaction_clap },
	{ id: 'fire', emoji: '🔥', label: m.party_reaction_fire }
];

const EMOJI = Object.fromEntries(REACTIONS.map((r) => [r.id, r.emoji])) as Record<
	RoomReaction,
	string
>;

export function reactionEmoji(reaction: RoomReaction): string {
	return EMOJI[reaction];
}

/** Whether `message` continues the one before it: same author, within a few minutes. */
export function continues(
	previous: { name: string; own: boolean; sent_at: number } | undefined,
	message: { name: string; own: boolean; sent_at: number }
): boolean {
	return (
		previous !== undefined &&
		previous.name === message.name &&
		previous.own === message.own &&
		message.sent_at - previous.sent_at < 3 * 60_000
	);
}
