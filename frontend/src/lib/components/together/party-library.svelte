<script lang="ts">
	import LibraryBigIcon from '@lucide/svelte/icons/library-big';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Popover from '#lib/components/ui/popover/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import PartyLibraryContent from './party-library-content.svelte';

	let {
		queued,
		onadd,
		onaddmany
	}: {
		queued: Set<string>;
		onadd: (videoId: string) => void;
		onaddmany: (videoIds: string[]) => void;
	} = $props();

	let open = $state(false);
</script>

<Popover.Root bind:open>
	<Popover.Trigger>
		{#snippet child({ props })}
			<Button
				{...props}
				variant="ghost"
				size="sm"
				class="whitespace-nowrap text-muted-foreground hover:text-foreground"
				data-testid="party-library-open"
			>
				<LibraryBigIcon />
				{m.party_library()}
			</Button>
		{/snippet}
	</Popover.Trigger>
	<Popover.Content align="start" class="w-[min(20rem,calc(100vw-2rem))] p-1">
		<PartyLibraryContent
			{queued}
			{onadd}
			onaddmany={(ids) => {
				onaddmany(ids);
				open = false;
			}}
		/>
	</Popover.Content>
</Popover.Root>
