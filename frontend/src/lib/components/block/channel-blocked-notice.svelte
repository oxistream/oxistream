<script lang="ts">
	import BanIcon from '@lucide/svelte/icons/ban';
	import { resolve } from '$app/paths';
	import ChannelAvatar from '#lib/components/channel/channel-avatar.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import type { ChannelSummary } from '#lib/video.js';

	let {
		channel,
		pending = false,
		onunblock,
		class: className
	}: {
		channel: Pick<ChannelSummary, 'name' | 'avatarUrl'>;
		/** The unblock request is on its way. */
		pending?: boolean;
		onunblock?: () => void;
		class?: string;
	} = $props();
</script>

<section
	class={cn(
		'flex flex-col items-center gap-5 rounded-xl border border-dashed px-6 py-12 text-center',
		className
	)}
	aria-labelledby="channel-blocked-title"
	data-testid="channel-blocked"
>
	<div class="relative">
		<ChannelAvatar
			name={channel.name}
			src={channel.avatarUrl}
			class="size-16 opacity-60 grayscale"
			fallbackClass="text-xl"
		/>
		<span
			class="absolute -end-1 -bottom-1 flex size-7 items-center justify-center rounded-full border-2 border-background bg-muted text-muted-foreground"
		>
			<BanIcon class="size-3.5" aria-hidden="true" />
		</span>
	</div>
	<h1 id="channel-blocked-title" class="max-w-md text-lg font-semibold tracking-tight text-balance">
		{m.block_notice_title({ name: channel.name })}
	</h1>
	<div class="flex flex-wrap justify-center gap-2">
		<Button
			onclick={onunblock}
			disabled={pending}
			aria-busy={pending || undefined}
			data-testid="unblock-button"
		>
			{m.block_unblock_channel()}
		</Button>
		<Button href={resolve('/settings/blocked')} variant="outline">{m.block_manage()}</Button>
	</div>
</section>
