import { FINISHED_RATIO, type ProgressReport } from '#lib/components/player/progress.js';

/**
 * Shorts loop, so playback never ends and the position falls back to the start. Once a short
 * got far enough, every later report of it says finished.
 */
export function createLoopingReporter(report: (report: ProgressReport) => void) {
	let finished = false;
	return {
		/** Call on every time update. */
		update: (position: number, duration: number) => {
			if (duration > 0 && position / duration >= FINISHED_RATIO) finished = true;
		},
		report: (next: ProgressReport) => {
			finished ||= next.finished;
			report({ ...next, finished });
		}
	};
}
