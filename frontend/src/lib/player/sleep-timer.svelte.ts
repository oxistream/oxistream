export type SleepMode = 'off' | '15' | '30' | '45' | '60' | 'end';

export const SLEEP_MODES: SleepMode[] = ['off', '15', '30', '45', '60', 'end'];

export function isSleepMode(value: unknown): value is SleepMode {
	return typeof value === 'string' && (SLEEP_MODES as string[]).includes(value);
}

/**
 * Pauses playback after a number of minutes or at the end of the current video, then turns
 * itself off. Minutes run on the wall clock, also while paused.
 */
export class SleepTimer {
	mode = $state<SleepMode>('off');
	/** `Date.now()` at which it fires, for minute modes. */
	deadline = $state<number | null>(null);
	#timer: ReturnType<typeof setTimeout> | undefined;
	#onexpire: () => void;
	#now: () => number;

	constructor(onexpire: () => void, now: () => number = Date.now) {
		this.#onexpire = onexpire;
		this.#now = now;
	}

	set(mode: SleepMode) {
		clearTimeout(this.#timer);
		this.#timer = undefined;
		this.mode = mode;
		this.deadline = null;
		if (mode === 'off' || mode === 'end') return;
		const ms = Number(mode) * 60_000;
		this.deadline = this.#now() + ms;
		this.#timer = setTimeout(() => this.#expire(), ms);
	}

	/** Call when a video ends. Returns whether the timer stops playback here. */
	ended(): boolean {
		if (this.mode !== 'end') return false;
		this.#expire();
		return true;
	}

	/** Whole minutes left, rounded up, for minute modes. */
	remaining(): number | null {
		if (this.deadline === null) return null;
		return Math.max(0, Math.ceil((this.deadline - this.#now()) / 60_000));
	}

	#expire() {
		this.set('off');
		this.#onexpire();
	}
}
