<script lang="ts">
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import ListXIcon from '@lucide/svelte/icons/list-x';
	import type { BlockChange } from '#lib/api/blocks.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';
	import QueueMenuItems from '#lib/components/queue/queue-menu-items.svelte';
	import AddToPartyMenuItem from '#lib/components/together/add-to-party-menu-item.svelte';
	import NotInterestedMenuItem from '#lib/components/video/not-interested-menu-item.svelte';
	import WatchedMenuItem from '#lib/components/video/watched-menu-item.svelte';
	import type { QueueVideo } from '#lib/player/queue.svelte.js';
	import type { VideoSummary } from '#lib/video.js';
	import BlockMenuItem from './block-menu-item.svelte';

	let {
		channel,
		label = m.block_more_actions(),
		variant = 'default',
		size = 'icon-sm',
		video,
		hideable = false,
		removeLabel,
		onremove,
		onblocked,
		class: className
	}: {
		channel: BlockChange['channel'];
		label?: string;
		/** `overlay` sits over video, as in the shorts player. */
		variant?: 'default' | 'overlay';
		size?: 'icon-sm' | 'icon' | 'icon-lg';
		/**
		 * Offers queueing this video, or adding it to the party this tab is in, and to signed-in
		 * users marking it as watched.
		 */
		video?: QueueVideo & Pick<VideoSummary, 'isLive' | 'isShort' | 'isUpcoming'>;
		/** Offers "Not interested", for listings that leave hidden videos out. */
		hideable?: boolean;
		/** Offers removing the video from the list it is shown in. */
		removeLabel?: string;
		onremove?: () => void;
		onblocked?: () => void;
		class?: string;
	} = $props();

	const removable = $derived(removeLabel !== undefined && onremove !== undefined);
	const blockable = $derived(session.user !== null && channel.id !== '');
	// Shorts and streams have no history.
	const markable = $derived(
		session.user !== null &&
			video !== undefined &&
			!video.isShort &&
			!video.isLive &&
			!video.isUpcoming
	);
	const notInterested = $derived(session.user !== null && video !== undefined && hideable);
	const personal = $derived(markable || notInterested || removable);
</script>

{#if video || removable || blockable}
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
		<DropdownMenu.Content align="end" class="w-max min-w-52">
			{#if video}
				<QueueMenuItems {video} />
				<AddToPartyMenuItem videoId={video.id} />
				{#if personal || blockable}<DropdownMenu.Separator />{/if}
			{/if}
			{#if markable && video}
				<WatchedMenuItem {video} />
			{/if}
			{#if notInterested && video}
				<NotInterestedMenuItem videoId={video.id} />
			{/if}
			{#if removable}
				<DropdownMenu.Item onSelect={onremove} data-testid="remove-from-list">
					<ListXIcon />
					{removeLabel}
				</DropdownMenu.Item>
			{/if}
			{#if personal && blockable}<DropdownMenu.Separator />{/if}
			{#if blockable}
				<BlockMenuItem {channel} {onblocked} />
			{/if}
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/if}
