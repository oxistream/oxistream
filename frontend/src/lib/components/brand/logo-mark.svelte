<script lang="ts">
	import type { SVGAttributes } from 'svelte/elements';

	let { class: className, ...restProps }: SVGAttributes<SVGSVGElement> = $props();
</script>

<!-- A rounded hex nut: oxide on iron, and the "O" of Oxistream. -->
<svg
	viewBox="0 0 24 24"
	fill="currentColor"
	aria-hidden="true"
	focusable="false"
	class={className}
	{...restProps}
>
	<path
		fill-rule="evenodd"
		d="M10.09 3.1Q12 2 13.91 3.1L18.75 5.9Q20.66 7 20.66 9.2L20.66 14.8Q20.66 17 18.75 18.1L13.91 20.9Q12 22 10.09 20.9L5.25 18.1Q3.34 17 3.34 14.8L3.34 9.2Q3.34 7 5.25 5.9ZM12 8a4 4 0 1 1 0 8a4 4 0 1 1 0-8Z"
	/>
</svg>
