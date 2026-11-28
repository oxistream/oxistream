/** A WebVTT cue; `text` keeps its tags and line breaks. */
export type Cue = { start: number; end: number; text: string };

export type TranscriptLine = { start: number; end: number; text: string };

const TIMESTAMP = /(?:(\d+):)?(\d{1,2}):(\d{2})[.,](\d{1,3})/;
const TIMING = new RegExp(`^\\s*${TIMESTAMP.source}\\s*-->\\s*${TIMESTAMP.source}`);
/** Karaoke timing inside a cue, as in YouTube's auto-generated captions. */
const INLINE_TIMESTAMP = /<(?:\d+:)?\d{1,2}:\d{2}[.,]\d{1,3}>/;

function seconds(h: string | undefined, m: string, s: string, ms: string): number {
	return Number(h ?? 0) * 3600 + Number(m) * 60 + Number(s) + Number(ms.padEnd(3, '0')) / 1000;
}

/** Reads the cues of a WebVTT file, skipping its header, notes, styles and regions. */
export function parseVtt(source: string): Cue[] {
	const lines = source.replace(/^\uFEFF/, '').split(/\r\n|\r|\n/);
	const cues: Cue[] = [];
	let cue: { start: number; end: number; lines: string[] } | null = null;

	const finish = () => {
		if (cue) cues.push({ start: cue.start, end: cue.end, text: cue.lines.join('\n') });
		cue = null;
	};

	// Lines outside cues (header, notes, styles, regions, identifiers) never hold a timing line.
	for (const line of lines) {
		const timing = TIMING.exec(line);
		if (timing) {
			// Without a blank line in between, the line before was the next cue's identifier.
			if (cue && cue.lines.length > 0) cue.lines.pop();
			finish();
			const [, h1, m1, s1, ms1, h2, m2, s2, ms2] = timing;
			cue = { start: seconds(h1, m1, s1, ms1), end: seconds(h2, m2, s2, ms2), lines: [] };
		} else if (line === '') {
			// Auto-captions start cues with a line holding a space, which some tools strip.
			if (cue?.lines.length === 0) continue;
			finish();
		} else if (cue) {
			cue.lines.push(line);
		}
	}
	finish();
	return cues.sort((a, b) => a.start - b.start);
}

const ENTITIES: Record<string, string> = {
	amp: '&',
	lt: '<',
	gt: '>',
	quot: '"',
	apos: "'",
	nbsp: ' ',
	lrm: '',
	rlm: ''
};

function decodeEntities(text: string): string {
	return text.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (match, name: string) => {
		if (name[0] === '#') {
			const code =
				name[1] === 'x' || name[1] === 'X'
					? parseInt(name.slice(2), 16)
					: parseInt(name.slice(1), 10);
			return Number.isFinite(code) && code > 0 && code <= 0x10ffff
				? String.fromCodePoint(code)
				: match;
		}
		return ENTITIES[name.toLowerCase()] ?? match;
	});
}

/** One line of cue text as plain text: no tags, entities decoded, whitespace collapsed. */
export function plainText(line: string): string {
	return decodeEntities(line.replace(/<[^>]*>/g, ''))
		.replace(/\s+/g, ' ')
		.trim();
}

/**
 * Transcript lines from cues. Auto-generated captions roll: each cue repeats the line before
 * it and the new line has karaoke timing, so only new lines are kept. Other cues become one
 * line each, and repeats of the same text in a row are merged.
 */
export function toTranscript(cues: Cue[]): TranscriptLine[] {
	const rolling = cues.some((c) => INLINE_TIMESTAMP.test(c.text));
	const out: TranscriptLine[] = [];
	const push = (start: number, end: number, text: string, timed: boolean) => {
		const last = out.at(-1);
		if (last && !timed && last.text === text) {
			// A rolled-over line is no longer the one being spoken.
			if (!rolling) last.end = Math.max(last.end, end);
			return;
		}
		if (last && !timed && text.startsWith(`${last.text} `) && start <= last.end) {
			last.text = text;
			last.end = Math.max(last.end, end);
			return;
		}
		out.push({ start, end, text });
	};

	for (const cue of cues) {
		const lines = cue.text.split('\n');
		if (rolling) {
			for (const raw of lines) {
				const text = plainText(raw);
				if (text) push(cue.start, cue.end, text, INLINE_TIMESTAMP.test(raw));
			}
		} else {
			const text = lines.map(plainText).filter(Boolean).join(' ');
			if (text) push(cue.start, cue.end, text, false);
		}
	}
	return out;
}

type Track = { id: string; lang: string; auto_generated: boolean };

const baseLang = (lang: string) => lang.split('-')[0].toLowerCase();

/**
 * The track a transcript opens with: the captions showing in the player, else the original
 * language (of the default audio track, else of the auto-generated captions), else the
 * auto-generated captions.
 */
export function defaultTrack<T extends Track>(
	tracks: readonly T[],
	{ showing, audioLang }: { showing?: string | null; audioLang?: string | null } = {}
): T | undefined {
	const current = tracks.find((t) => t.id === showing);
	if (current) return current;
	const auto = tracks.find((t) => t.auto_generated);
	const original = audioLang ?? auto?.lang;
	const manual = original
		? tracks.find((t) => !t.auto_generated && baseLang(t.lang) === baseLang(original))
		: undefined;
	return manual ?? auto ?? tracks[0];
}

/** Indexes of the lines containing `query`, ignoring case. */
export function findMatches(lines: readonly TranscriptLine[], query: string): number[] {
	const needle = query.trim().toLocaleLowerCase();
	if (!needle) return [];
	const found: number[] = [];
	lines.forEach((line, i) => {
		if (line.text.toLocaleLowerCase().includes(needle)) found.push(i);
	});
	return found;
}

/** `text` split around every occurrence of `query`, ignoring case. */
export function highlight(text: string, query: string): { text: string; match: boolean }[] {
	const needle = query.trim().toLocaleLowerCase();
	if (!needle) return [{ text, match: false }];
	const lower = text.toLocaleLowerCase();
	// Lowercasing can change the length, e.g. of "İ"; offsets would no longer line up.
	if (lower.length !== text.length) return [{ text, match: false }];
	const parts: { text: string; match: boolean }[] = [];
	let from = 0;
	for (let at = lower.indexOf(needle); at !== -1; at = lower.indexOf(needle, from)) {
		if (at > from) parts.push({ text: text.slice(from, at), match: false });
		parts.push({ text: text.slice(at, at + needle.length), match: true });
		from = at + needle.length;
	}
	if (from < text.length) parts.push({ text: text.slice(from), match: false });
	return parts;
}
