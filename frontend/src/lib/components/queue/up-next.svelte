<script lang="ts">
	import ListXIcon from '@lucide/svelte/icons/list-x';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { appPlayer } from '#lib/player/app-player.svelte.js';
	import { cn } from '#lib/utils.js';
	import QueueList from './queue-list.svelte';

	let { class: className, listClass }: { class?: string; listClass?: string } = $props();

	const headingId = $props.id();
</script>

<section
	aria-labelledby={headingId}
	class={cn('flex flex-col gap-1 rounded-xl border bg-card p-1.5', className)}
	data-testid="up-next"
>
	<header class="flex items-center gap-2 ps-2.5 pe-0.5">
		<h2 id={headingId} class="flex-1 text-sm font-semibold tracking-tight">
			{m.queue_heading()}
			<span class="ms-1 font-normal text-muted-foreground tabular-nums">
				{appPlayer.queue.length}
			</span>
		</h2>
		<Button
			variant="ghost"
			size="icon-sm"
			class="text-muted-foreground hover:text-foreground"
			aria-label={m.queue_clear()}
			title={m.queue_clear()}
			onclick={() => appPlayer.queue.clear()}
			data-testid="queue-clear"
		>
			<ListXIcon />
		</Button>
	</header>
	<QueueList class={cn('max-h-80 overflow-y-auto', listClass)} />
</section>
