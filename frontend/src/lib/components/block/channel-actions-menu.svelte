<script lang="ts">
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import type { BlockChange } from '#lib/api/blocks.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import AddToRoomMenuItem from '#lib/components/together/add-to-room-menu-item.svelte';
	import BlockMenuItem from './block-menu-item.svelte';

	let {
		channel,
		label = m.block_more_actions(),
		variant = 'default',
		size = 'icon-sm',
		videoId,
		onblocked,
		class: className
	}: {
		channel: BlockChange['channel'];
		label?: string;
		/** `overlay` sits over video, as in the shorts player. */
		variant?: 'default' | 'overlay';
		size?: 'icon-sm' | 'icon' | 'icon-lg';
		/** Offers adding this video to a room open in another tab. */
		videoId?: string;
		onblocked?: () => void;
		class?: string;
	} = $props();
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class={cn(
			buttonVariants({ variant: 'ghost', size }),
			variant === 'overlay'
				? 'rounded-full bg-black/35 text-white backdrop-blur-sm hover:bg-white/20 hover:text-white data-[state=open]:bg-white/20'
				: 'rounded-full text-muted-foreground hover:text-foreground data-[state=open]:bg-accent data-[state=open]:text-foreground',
			className
		)}
		aria-label={label}
		title={label}
		data-testid="channel-actions"
	>
		<EllipsisVerticalIcon />
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end" class="min-w-52">
		{#if videoId}<AddToRoomMenuItem {videoId} />{/if}
		<BlockMenuItem {channel} {onblocked} />
	</DropdownMenu.Content>
</DropdownMenu.Root>
