export type ProgressReason = 'interval' | 'pause' | 'ended' | 'unload';

export type ProgressReport = {
	/** Seconds into the video. */
	position: number;
	duration: number;
	/** At least 90 % watched, or played to the end. */
	finished: boolean;
	reason: ProgressReason;
};

export const PROGRESS_INTERVAL_MS = 10_000;
export const FINISHED_RATIO = 0.9;

/**
 * Turns a stream of playback updates into progress reports: one every `interval` while playing,
 * plus one on pause, end and unload. Repeats of the last reported position are dropped.
 */
export function createProgressEmitter(
	onprogress: (report: ProgressReport) => void,
	{ interval = PROGRESS_INTERVAL_MS, now = () => Date.now() } = {}
) {
	let lastEmit: number | null = null;
	let lastPosition: number | null = null;

	function emit(position: number, duration: number, reason: ProgressReason) {
		if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0) return;
		const rounded = Math.round(position * 10) / 10;
		lastEmit = now();
		if (rounded === lastPosition) return;
		lastPosition = rounded;
		onprogress({
			position: rounded,
			duration,
			finished: reason === 'ended' || position / duration >= FINISHED_RATIO,
			reason
		});
	}

	return {
		/** Call on every time update. */
		update(position: number, duration: number, playing: boolean) {
			if (!playing) return;
			if (lastEmit === null) lastEmit = now();
			else if (now() - lastEmit >= interval) emit(position, duration, 'interval');
		},
		/** Report right away, e.g. on pause or when the page goes away. */
		flush(position: number, duration: number, reason: Exclude<ProgressReason, 'interval'>) {
			emit(position, duration, reason);
		}
	};
}
