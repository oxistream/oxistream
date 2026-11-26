export type Step = 1 | -1;

const EDITABLE =
	'input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="slider"], [role="menu"], [role="menuitem"], [role="menuitemradio"], [role="radiogroup"], [role="listbox"]';

/** The step a key press asks for: ArrowDown and j go to the next short, ArrowUp and k back. */
export function keyStep(event: {
	key: string;
	ctrlKey: boolean;
	metaKey: boolean;
	altKey: boolean;
	shiftKey: boolean;
	target: EventTarget | null;
}): Step | null {
	if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return null;
	const target = event.target as Element | null;
	if (target && typeof target.closest === 'function' && target.closest(EDITABLE)) return null;
	switch (event.key) {
		case 'ArrowDown':
		case 'j':
			return 1;
		case 'ArrowUp':
		case 'k':
			return -1;
		default:
			return null;
	}
}

const WHEEL_THRESHOLD = 40;
// Trackpads keep sending decaying events after a flick; a pause this long ends the gesture.
const WHEEL_GAP_MS = 200;
const LINE_HEIGHT = 16;

/** Turns wheel events into steps: one per gesture, so a single flick moves one short. */
export function createWheelPager(onstep: (step: Step) => void) {
	let total = 0;
	let locked = false;
	let lastEvent = -Infinity;

	return (event: { deltaY: number; deltaMode: number; timeStamp: number }) => {
		const gap = event.timeStamp - lastEvent;
		lastEvent = event.timeStamp;
		if (gap > WHEEL_GAP_MS) {
			locked = false;
			total = 0;
		}
		if (locked) return;
		total += event.deltaMode === 1 ? event.deltaY * LINE_HEIGHT : event.deltaY;
		if (Math.abs(total) < WHEEL_THRESHOLD) return;
		onstep(total > 0 ? 1 : -1);
		locked = true;
		total = 0;
	};
}

/** Movement before a touch counts as a drag instead of a tap. */
export const DRAG_SLOP_PX = 8;
const SNAP_RATIO = 0.25;
const FLICK_PX_PER_MS = 0.4;
const FLICK_MIN_PX = 24;
// Dragging past the first or last short only moves this fraction of the finger's way.
const EDGE_RESISTANCE = 0.25;

/** Whether a touch that moved this far has become a vertical drag. */
export function isVerticalDrag(dx: number, dy: number): boolean {
	return Math.abs(dy) > DRAG_SLOP_PX && Math.abs(dy) > Math.abs(dx);
}

/** How far the shorts move for a finger offset, with resistance where nothing follows. */
export function dragOffset(dy: number, canStep: (step: Step) => boolean): number {
	const step: Step = dy < 0 ? 1 : -1;
	return canStep(step) ? dy : dy * EDGE_RESISTANCE;
}

/**
 * Where a released drag goes: far enough or flicked fast enough moves one short (up shows the
 * next one), anything else springs back.
 */
export function releaseStep(dy: number, velocity: number, height: number): Step | null {
	const far = Math.abs(dy) >= height * SNAP_RATIO;
	const flick =
		Math.abs(dy) >= FLICK_MIN_PX &&
		Math.abs(velocity) >= FLICK_PX_PER_MS &&
		Math.sign(velocity) === Math.sign(dy);
	if (!far && !flick) return null;
	return dy < 0 ? 1 : -1;
}
