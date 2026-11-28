<script lang="ts">
	import type { SubscriptionGroup } from '#lib/api/subscription-group-list.js';
	import { tabLinkClass } from '#lib/components/tab-link.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		groups,
		group,
		href,
		class: className
	}: {
		groups: SubscriptionGroup[];
		/** ID of the selected group; all channels without one. */
		group?: string;
		/** Link to a group, or to all channels without an ID. */
		href: (id: string | undefined) => string;
		class?: string;
	} = $props();
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve -- resolved by the caller -->
<div class={cn('min-w-0 border-b', className)}>
	<nav aria-label={m.groups_label()} class="-mb-px overflow-x-auto" data-testid="group-tabs">
		<ul class="flex gap-1">
			<li class="shrink-0">
				<a
					href={href(undefined)}
					aria-current={group ? undefined : 'page'}
					data-sveltekit-noscroll
					class={tabLinkClass}
				>
					{m.groups_all()}
				</a>
			</li>
			{#each groups as item (item.id)}
				<li class="shrink-0">
					<a
						href={href(item.id)}
						aria-current={item.id === group ? 'page' : undefined}
						data-sveltekit-noscroll
						class={cn(tabLinkClass, 'max-w-56')}
						data-testid="group-tab"
					>
						<span class="truncate">{item.name}</span>
					</a>
				</li>
			{/each}
		</ul>
	</nav>
</div>
<!-- eslint-enable svelte/no-navigation-without-resolve -->
