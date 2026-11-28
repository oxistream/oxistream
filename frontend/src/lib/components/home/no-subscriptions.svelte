<script lang="ts">
	import ArrowDownUpIcon from '@lucide/svelte/icons/arrow-down-up';
	import SearchIcon from '@lucide/svelte/icons/search';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { resolve } from '$app/paths';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
</script>

<EmptyState icon={TvMinimalPlayIcon} title={m.empty_subscriptions_title()} headingLevel={3}>
	{#snippet actions()}
		<Button href={resolve('/results')}>
			<SearchIcon />
			{m.home_find_channels()}
		</Button>
		<Button href={resolve('/subscriptions/import-export')} variant="outline">
			<ArrowDownUpIcon />
			{m.subscriptions_import_export()}
		</Button>
	{/snippet}
</EmptyState>
