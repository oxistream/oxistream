<script lang="ts">
	import { cn } from '#lib/utils.js';
	import { qrSvg } from './qr.js';

	let {
		value,
		label,
		class: className
	}: { value: string; label: string; class?: string } = $props();

	const qr = $derived(qrSvg(value));
</script>

<!-- Always dark on white: many scanners cannot read inverted codes. -->
<svg
	viewBox="0 0 {qr.size} {qr.size}"
	role="img"
	aria-label={label}
	shape-rendering="crispEdges"
	class={cn('block aspect-square w-full rounded-lg', className)}
	data-testid="party-qr"
	data-value={value}
>
	<rect width={qr.size} height={qr.size} fill="#fff" />
	<path d={qr.path} fill="#000" />
</svg>
