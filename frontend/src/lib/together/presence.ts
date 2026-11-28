import type { MemberPlace, MemberStatus, MemberSync, RoomParticipant } from './connection.svelte';
import { PLAYING_DRIFT } from './sync';

/** A sync state must hold this long before it is reported, so short stalls do not flicker. */
export const STATUS_HOLD_MS = 1000;
/** Reports go out at most this often. */
export const STATUS_INTERVAL_MS = 2000;
/** Someone who comes back this soon, e.g. after a reload, neither left nor joined. */
export const REJOIN_MS = 5000;
/** Joins or leaves this close together make one notice. */
export const COALESCE_MS = 2000;
export const NOTICE_MS = 5000;

/** How a sync state of the party player counts for the members list. */
export function syncHealth(state: string, drift: number | null): MemberSync | undefined {
	switch (state) {
		case 'paused':
		case 'locked':
		case 'catching up':
			return drift !== null && Math.abs(drift) > PLAYING_DRIFT ? 'behind' : 'synced';
		case 'seeking':
		case 'starting':
		case 'settling':
		case 'buffering':
			return 'buffering';
		case 'blocked':
			return 'behind';
		default:
			// Reporting or scrubbing: what this player does is on its way to the others.
			return undefined;
	}
}

/** Where this tab is, from its route. */
export function placeOf(
	routeId: string | null,
	videoParam: string | null,
	partyVideo: string | null
): MemberPlace {
	if (routeId?.startsWith('/shorts')) return 'shorts';
	if (routeId === '/watch' && partyVideo !== null && videoParam === partyVideo) return 'watching';
	return 'browsing';
}

type Clock = { now: () => number };

/** Sends this tab's status when it changes: sync states once they held, at most every 2 s. */
export class StatusReporter {
	#send: (status: MemberStatus) => void;
	#clock: Clock;
	#sent: MemberStatus | null = null;
	#sentAt = -Infinity;
	#shown: MemberStatus | null = null;
	#candidate: MemberSync | null = null;
	#since = 0;
	#timer: ReturnType<typeof setTimeout> | undefined;

	constructor(send: (status: MemberStatus) => void, clock: Clock = { now: () => Date.now() }) {
		this.#send = send;
		this.#clock = clock;
	}

	update(status: MemberStatus) {
		const now = this.#clock.now();
		if (status.sync !== this.#candidate) {
			this.#candidate = status.sync;
			this.#since = now;
		}
		const held = now - this.#since >= STATUS_HOLD_MS;
		// Nothing to play, or nothing shown yet, needs no hold.
		const sync =
			held || status.sync === null || !this.#shown ? status.sync : (this.#shown.sync ?? null);
		this.#shown = { sync, place: status.place };
		this.#flush(status);
	}

	destroy() {
		clearTimeout(this.#timer);
	}

	#flush(latest: MemberStatus) {
		clearTimeout(this.#timer);
		const now = this.#clock.now();
		const shown = this.#shown!;
		let wait = 0;
		if (shown.sync !== latest.sync) wait = this.#since + STATUS_HOLD_MS - now;
		const changed = shown.sync !== this.#sent?.sync || shown.place !== this.#sent?.place;
		if (changed) {
			const due = this.#sentAt + STATUS_INTERVAL_MS - now;
			if (due <= 0) {
				this.#sent = shown;
				this.#sentAt = now;
				this.#send(shown);
			} else wait = wait > 0 ? Math.min(wait, due) : due;
		}
		if (wait > 0) this.#timer = setTimeout(() => this.update(latest), wait);
	}
}

type Person = Pick<RoomParticipant, 'id' | 'name' | 'is_guest'>;
export type PresenceKind = 'joined' | 'left';
export type PresenceNotice = { kind: PresenceKind; names: string[]; at: number };

const who = (p: Person) => `${p.is_guest ? 'g' : 'u'}:${p.name}`;

/**
 * Short notices when people join or leave. Reconnects and reloads, which come back within a
 * few seconds as a new participant, count as neither; neither does joining yourself.
 */
export class PresenceNotices {
	#notice: PresenceNotice | null = null;
	#onchange: (notice: PresenceNotice | null) => void;
	#clock: Clock;
	#me: string | null = null;
	#present = new Map<string, Person>();
	#leaving = new Map<string, { person: Person; timer: ReturnType<typeof setTimeout> }>();
	#hide: ReturnType<typeof setTimeout> | undefined;

	constructor(
		onchange: (notice: PresenceNotice | null) => void,
		clock: Clock = { now: () => Date.now() }
	) {
		this.#onchange = onchange;
		this.#clock = clock;
	}

	/** The participants of the latest state, as participant `me`. */
	update(participants: Person[], me: string | null) {
		const present = new Map(participants.map((p) => [p.id, p]));
		// After joining or reconnecting, everyone already there is no news.
		if (me !== this.#me) {
			this.#me = me;
			this.#present = present;
			return;
		}
		const names = new Set(participants.map(who));
		for (const [id, person] of present) {
			if (this.#present.has(id) || id === me) continue;
			const key = who(person);
			const back = this.#leaving.get(key);
			if (back) {
				clearTimeout(back.timer);
				this.#leaving.delete(key);
				continue;
			}
			// Still there under an earlier connection that is yet to time out.
			if ([...this.#present.values()].some((p) => who(p) === key)) continue;
			this.#announce('joined', person.name);
		}
		for (const [id, person] of this.#present) {
			if (present.has(id) || id === me) continue;
			const key = who(person);
			if (names.has(key) || this.#leaving.has(key)) continue;
			const timer = setTimeout(() => {
				this.#leaving.delete(key);
				this.#announce('left', person.name);
			}, REJOIN_MS);
			this.#leaving.set(key, { person, timer });
		}
		this.#present = present;
	}

	destroy() {
		clearTimeout(this.#hide);
		for (const { timer } of this.#leaving.values()) clearTimeout(timer);
		this.#leaving.clear();
	}

	#announce(kind: PresenceKind, name: string) {
		const now = this.#clock.now();
		const last = this.#notice;
		const names =
			last && last.kind === kind && now - last.at < COALESCE_MS
				? [...last.names.filter((n) => n !== name), name]
				: [name];
		this.#set({ kind, names, at: now });
		clearTimeout(this.#hide);
		this.#hide = setTimeout(() => this.#set(null), NOTICE_MS);
	}

	#set(notice: PresenceNotice | null) {
		this.#notice = notice;
		this.#onchange(notice);
	}
}
