<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { party } from '#lib/together/party.svelte.js';
	import { cn } from '#lib/utils.js';
	import { addErrorMessage } from './errors.js';

	let { videoId, class: className }: { videoId: string; class?: string } = $props();

	let sentAt = $state<number | null>(null);

	// A different video or party starts over.
	$effect(() => {
		void [videoId, party.id];
		sentAt = null;
	});

	const playing = $derived(party.current?.video.id === videoId);
	const queued = $derived(party.room?.queue.some((e) => e.video.id === videoId) ?? false);
	$effect(() => {
		if (queued) sentAt = null;
	});
	const error = $derived.by(() => {
		const last = party.connection?.lastError;
		return sentAt !== null && last && last.at >= sentAt ? addErrorMessage(last.code) : null;
	});

	function add() {
		sentAt = Date.now();
		party.add(videoId);
	}
</script>

{#if party.active && !playing}
	<Button
		variant="secondary"
		size="sm"
		class={cn('rounded-full px-3.5', className)}
		disabled={party.status !== 'open' || (sentAt !== null && !error) || queued}
		onclick={add}
		data-testid="add-to-party"
	>
		{#if queued}
			<CheckIcon />
			{m.party_added()}
		{:else}
			<ListPlusIcon />
			{m.party_add()}
		{/if}
	</Button>
	{#if error}
		<p class="basis-full text-xs font-medium text-destructive" role="alert">{error}</p>
	{/if}
{/if}
