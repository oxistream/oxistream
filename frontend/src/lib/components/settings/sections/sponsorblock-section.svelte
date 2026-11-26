<script lang="ts">
	import SponsorBlockPreferences from '#lib/components/sponsorblock/sponsorblock-preferences.svelte';
	import { m } from '#lib/paraglide/messages.js';
</script>

<div class="flex flex-col gap-4 px-5 py-4">
	<p class="text-sm text-pretty text-muted-foreground">{m.sponsorblock_description()}</p>
	<SponsorBlockPreferences header={false} />
</div>
