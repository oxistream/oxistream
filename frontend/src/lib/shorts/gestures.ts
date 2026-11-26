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

const SWIPE_MIN_PX = 56;

/** A mostly vertical swipe: up shows the next short, down the previous one. */
export function swipeStep(
	start: { x: number; y: number },
	end: { x: number; y: number }
): Step | null {
	const dx = end.x - start.x;
	const dy = end.y - start.y;
	if (Math.abs(dy) < SWIPE_MIN_PX || Math.abs(dy) < Math.abs(dx) * 1.5) return null;
	return dy < 0 ? 1 : -1;
}
