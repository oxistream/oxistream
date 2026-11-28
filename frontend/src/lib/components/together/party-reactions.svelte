<script lang="ts">
	import { reactionEmoji } from '#lib/together/chat.js';
	import { REACTION_MS, type FloatingReaction } from '#lib/together/party.svelte.js';
	import { cn } from '#lib/utils.js';

	let { reactions, class: className }: { reactions: FloatingReaction[]; class?: string } = $props();
</script>

<!-- Over the video; clicks pass through. -->
<div
	class={cn('party-reactions pointer-events-none absolute inset-0 overflow-hidden', className)}
	style:--reaction-ms="{REACTION_MS}ms"
	aria-hidden="true"
	data-testid="party-reactions"
>
	{#each reactions as item (item.key)}
		<span
			class="party-reaction"
			style:inset-inline-start="{item.x}%"
			data-testid="party-reaction"
			data-reaction={item.reaction}
		>
			{reactionEmoji(item.reaction)}
		</span>
	{/each}
</div>

<style>
	.party-reactions {
		container-type: size;
	}

	.party-reaction {
		position: absolute;
		bottom: 18%;
		font-size: clamp(1.25rem, 8cqmin, 2.75rem);
		line-height: 1;
		translate: -50% 0;
		opacity: 0;
		filter: drop-shadow(0 1px 2px rgb(0 0 0 / 0.4));
		animation: party-float var(--reaction-ms) ease-out forwards;
	}

	@keyframes party-float {
		0% {
			opacity: 0;
			transform: translateY(0) scale(0.6);
		}
		12% {
			opacity: 1;
			transform: translateY(-6cqh) scale(1.1);
		}
		70% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translateY(-55cqh) scale(1);
		}
	}

	@keyframes party-fade {
		0%,
		100% {
			opacity: 0;
		}
		15%,
		75% {
			opacity: 1;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.party-reaction {
			animation-name: party-fade;
			animation-timing-function: linear;
		}
	}
</style>
