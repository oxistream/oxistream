<script lang="ts">
	import * as Avatar from '#lib/components/ui/avatar/index.js';
	import { cn } from '#lib/utils.js';
	import { initials } from '#lib/video.js';

	let {
		name,
		src,
		size = 'default',
		class: className,
		fallbackClass
	}: {
		name: string;
		src?: string;
		size?: 'sm' | 'default' | 'lg';
		class?: string;
		/** Sizes the initials for avatars larger than `lg`. */
		fallbackClass?: string;
	} = $props();
</script>

<Avatar.Root {size} class={cn('bg-muted', className)}>
	{#if src}
		<!-- The name is always printed next to the avatar, so the image is decorative. -->
		<Avatar.Image {src} alt="" class="object-cover" />
	{/if}
	<Avatar.Fallback
		class={cn('bg-muted text-xs font-medium text-muted-foreground', fallbackClass)}
		aria-hidden="true"
	>
		{initials(name)}
	</Avatar.Fallback>
</Avatar.Root>
