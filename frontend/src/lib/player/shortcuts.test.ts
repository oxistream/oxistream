import { describe, expect, it } from 'vitest';
import { SHORTCUT_GROUPS, matchShortcut, shortcutsBlocked } from './shortcuts';

type Fake = EventTarget & {
	tagName: string;
	isContentEditable?: boolean;
	parent?: Fake;
	matches: string[];
	closest: (selector: string) => Fake | null;
};

/** Just enough of an element for `closest`, matching tag names and the listed selectors. */
function el(tagName: string, options: { parent?: Fake; matches?: string[] } = {}): Fake {
	const node = {
		tagName,
		parent: options.parent,
		matches: options.matches ?? [],
		closest(selector: string) {
			const parts = selector.split(',').map((s) => s.trim());
			for (let n: Fake | undefined = node; n; n = n.parent) {
				if (parts.some((s) => n.matches.includes(s) || s === n.tagName.toLowerCase())) return n;
			}
			return null;
		}
	} as Fake;
	return node;
}

const body = el('BODY');
const key = (
	k: string,
	extra: Omit<Partial<KeyboardEvent>, 'target'> & { target?: EventTarget | null } = {}
) =>
	({
		key: k,
		shiftKey: false,
		ctrlKey: false,
		altKey: false,
		metaKey: false,
		repeat: false,
		defaultPrevented: false,
		target: body,
		...extra
	}) as Parameters<typeof matchShortcut>[0];

describe('matchShortcut', () => {
	it('maps shift+N and shift+P to next and previous', () => {
		expect(matchShortcut(key('N', { shiftKey: true }))).toBe('next');
		expect(matchShortcut(key('P', { shiftKey: true }))).toBe('previous');
		// Caps lock gives lowercase with shift.
		expect(matchShortcut(key('n', { shiftKey: true }))).toBe('next');
	});

	it('needs shift for next and previous', () => {
		expect(matchShortcut(key('n'))).toBeNull();
		expect(matchShortcut(key('p'))).toBeNull();
	});

	it('opens the overlay on ? with or without shift', () => {
		expect(matchShortcut(key('?', { shiftKey: true }))).toBe('help');
		expect(matchShortcut(key('?'))).toBe('help');
	});

	it('ignores other modifiers, repeats and handled events', () => {
		expect(matchShortcut(key('N', { shiftKey: true, ctrlKey: true }))).toBeNull();
		expect(matchShortcut(key('N', { shiftKey: true, metaKey: true }))).toBeNull();
		expect(matchShortcut(key('?', { altKey: true }))).toBeNull();
		expect(matchShortcut(key('N', { shiftKey: true, repeat: true }))).toBeNull();
		expect(matchShortcut(key('?', { defaultPrevented: true }))).toBeNull();
	});

	it('leaves keys typed into fields alone', () => {
		expect(matchShortcut(key('?', { target: el('INPUT') }))).toBeNull();
		expect(matchShortcut(key('N', { shiftKey: true, target: el('TEXTAREA') }))).toBeNull();
	});

	it('works without a target', () => {
		expect(matchShortcut(key('?', { target: null }))).toBe('help');
	});

	it('leaves other keys to the player', () => {
		for (const k of ['k', ' ', 'j', 'l', 'm', 'f', 'c', '<', '>', '5', 'ArrowLeft']) {
			expect(matchShortcut(key(k))).toBeNull();
		}
	});
});

describe('shortcutsBlocked', () => {
	it('blocks text fields, selects and contenteditable', () => {
		expect(shortcutsBlocked(el('INPUT'))).toBe(true);
		expect(shortcutsBlocked(el('SELECT'))).toBe(true);
		const editable = el('DIV');
		editable.isContentEditable = true;
		expect(shortcutsBlocked(editable)).toBe(true);
	});

	it('blocks dialogs outside the player', () => {
		const dialog = el('DIV', { parent: body, matches: ['[role="dialog"]'] });
		expect(shortcutsBlocked(el('BUTTON', { parent: dialog }))).toBe(true);
	});

	it("allows the player's own popups", () => {
		const container = el('MEDIA-CONTAINER', { parent: body });
		const popup = el('DIV', { parent: container, matches: ['[role="dialog"]'] });
		expect(shortcutsBlocked(el('BUTTON', { parent: popup }))).toBe(false);
	});

	it('allows the page', () => {
		expect(shortcutsBlocked(el('BUTTON', { parent: body }))).toBe(false);
		expect(shortcutsBlocked(null)).toBe(false);
	});
});

describe('SHORTCUT_GROUPS', () => {
	it('lists every action once', () => {
		const actions = SHORTCUT_GROUPS.flatMap((g) => g.entries.map((e) => e.action));
		expect(new Set(actions).size).toBe(actions.length);
		expect(actions).toEqual(expect.arrayContaining(['play', 'next', 'previous', 'help', 'mute']));
	});
});
