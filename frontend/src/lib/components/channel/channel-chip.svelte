<script lang="ts">
	import { resolve } from '$app/paths';
	import { cn } from '#lib/utils.js';
	import type { ChannelSummary } from '#lib/video.js';
	import ChannelAvatar from './channel-avatar.svelte';

	let { channel, class: className }: { channel: ChannelSummary; class?: string } = $props();
</script>

<a
	href={resolve('/channel/[id]', { id: channel.id })}
	class={cn(
		'inline-flex h-8 max-w-full items-center gap-2 rounded-full border bg-background py-1 ps-1 pe-3 text-sm font-medium transition-colors duration-(--motion-in) ease-ui hover:bg-muted',
		className
	)}
>
	<ChannelAvatar name={channel.name} src={channel.avatarUrl} size="sm" />
	<span class="truncate">{channel.name}</span>
</a>
