<script lang="ts" module>
	export type SyncStats = {
		/** Server clock minus local clock, in ms. */
		offset: number;
		/** Shortest ping round trip in ms. */
		rtt: number | null;
		/** Local position minus the room's, in ms; `null` while not measured. */
		drift: number | null;
		rate: number;
		/** Seconds a seek aims ahead of the room. */
		lead: number;
		/** Seconds of playback lost per rate change. */
		rateCost: number;
		state: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { stats, class: className }: { stats: SyncStats; class?: string } = $props();

	const ms = (value: number | null, sign = false) =>
		value === null ? '–' : `${sign && value > 0 ? '+' : ''}${value.toFixed(1)} ms`;
</script>

<!-- Debug readout for ?debug=sync; not translated on purpose. -->
<dl
	class={cn(
		'pointer-events-none grid grid-cols-[auto_auto] gap-x-3 rounded-md bg-black/75 px-2.5 py-1.5 font-mono text-[11px] leading-4 text-white tabular-nums',
		className
	)}
	data-testid="sync-readout"
	data-drift={stats.drift ?? ''}
	data-rate={stats.rate}
>
	<dt class="text-white/60">state</dt>
	<dd>{stats.state}</dd>
	<dt class="text-white/60">drift</dt>
	<dd>{ms(stats.drift, true)}</dd>
	<dt class="text-white/60">rate</dt>
	<dd>{stats.rate.toFixed(3)}</dd>
	<dt class="text-white/60">offset</dt>
	<dd>{ms(stats.offset, true)}</dd>
	<dt class="text-white/60">rtt</dt>
	<dd>{ms(stats.rtt)}</dd>
	<dt class="text-white/60">seek lead</dt>
	<dd>{(stats.lead * 1000).toFixed(0)} ms</dd>
	<dt class="text-white/60">rate cost</dt>
	<dd>{(stats.rateCost * 1000).toFixed(0)} ms</dd>
</dl>
