/** Shortcuts handled by the app; the player's own (Video.js hotkeys) cover the rest. */
export type AppShortcut = 'next' | 'previous' | 'help';

type KeyEventLike = Pick<
	KeyboardEvent,
	'key' | 'shiftKey' | 'ctrlKey' | 'altKey' | 'metaKey' | 'repeat' | 'defaultPrevented'
> & { target: EventTarget | null };

type ElementLike = {
	tagName?: string;
	isContentEditable?: boolean;
	closest?: (selector: string) => ElementLike | null;
};

const DIALOG = '[role="dialog"], [role="alertdialog"], dialog[open]';
const EDITABLE = new Set(['INPUT', 'TEXTAREA', 'SELECT']);

function isEditable(el: ElementLike): boolean {
	return el.isContentEditable === true || EDITABLE.has(el.tagName ?? '');
}

/**
 * Whether keys on `target` belong to it rather than to the player: text fields and dialogs
 * outside the player, e.g. the shortcuts overlay or a sheet. The player's own popups count as the player.
 */
export function shortcutsBlocked(target: EventTarget | null): boolean {
	const el = target as ElementLike | null;
	if (!el || typeof el.closest !== 'function') return false;
	if (isEditable(el)) return true;
	const dialog = el.closest(DIALOG);
	return dialog !== null && dialog.closest?.('media-container') == null;
}

/** The app shortcut a key press asks for, if any. */
export function matchShortcut(event: KeyEventLike): AppShortcut | null {
	if (event.defaultPrevented || event.ctrlKey || event.altKey || event.metaKey) return null;
	if (shortcutsBlocked(event.target)) return null;
	// Shift is part of the character on most layouts (US Shift+/, German Shift+ß).
	if (event.key === '?') return event.repeat ? null : 'help';
	if (!event.shiftKey || event.repeat) return null;
	if (event.key === 'N' || event.key === 'n') return 'next';
	if (event.key === 'P' || event.key === 'p') return 'previous';
	return null;
}

export type ShortcutKey = { text: string } | { name: 'space' | 'home' | 'end' | 'shift' };

export type ShortcutAction =
	| 'play'
	| 'back10'
	| 'forward10'
	| 'back5'
	| 'forward5'
	| 'percent'
	| 'start'
	| 'end'
	| 'slower'
	| 'faster'
	| 'next'
	| 'previous'
	| 'volumeUp'
	| 'volumeDown'
	| 'mute'
	| 'fullscreen'
	| 'theater'
	| 'captions'
	| 'pip'
	| 'help';

export type ShortcutEntry = {
	action: ShortcutAction;
	/** Alternatives; each is a combination pressed together. */
	keys: ShortcutKey[][];
};

const k = (text: string): ShortcutKey => ({ text });
const shift: ShortcutKey = { name: 'shift' };

/** What the overlay lists, by group, matching the player's hotkeys and `matchShortcut`. */
export const SHORTCUT_GROUPS: { id: 'playback' | 'sound' | 'view'; entries: ShortcutEntry[] }[] = [
	{
		id: 'playback',
		entries: [
			{ action: 'play', keys: [[k('K')], [{ name: 'space' }]] },
			{ action: 'back10', keys: [[k('J')]] },
			{ action: 'forward10', keys: [[k('L')]] },
			{ action: 'back5', keys: [[k('←')]] },
			{ action: 'forward5', keys: [[k('→')]] },
			{ action: 'percent', keys: [[k('0–9')]] },
			{ action: 'start', keys: [[{ name: 'home' }]] },
			{ action: 'end', keys: [[{ name: 'end' }]] },
			{ action: 'slower', keys: [[k('<')]] },
			{ action: 'faster', keys: [[k('>')]] },
			{ action: 'previous', keys: [[shift, k('P')]] },
			{ action: 'next', keys: [[shift, k('N')]] }
		]
	},
	{
		id: 'sound',
		entries: [
			{ action: 'volumeUp', keys: [[k('↑')]] },
			{ action: 'volumeDown', keys: [[k('↓')]] },
			{ action: 'mute', keys: [[k('M')]] }
		]
	},
	{
		id: 'view',
		entries: [
			{ action: 'fullscreen', keys: [[k('F')]] },
			{ action: 'theater', keys: [[k('T')]] },
			{ action: 'captions', keys: [[k('C')]] },
			{ action: 'pip', keys: [[k('I')]] },
			{ action: 'help', keys: [[k('?')]] }
		]
	}
];
