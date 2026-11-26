<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';

	let { failed }: { failed: boolean } = $props();
</script>

{#if failed}
	<p role="alert" class="px-5 py-3 text-sm font-medium text-destructive">
		{m.settings_save_failed()}
	</p>
{/if}
