<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import LinkIcon from '@lucide/svelte/icons/link';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let { url, class: className }: { url: string; class?: string } = $props();

	let copied = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	async function copy() {
		try {
			await navigator.clipboard.writeText(url);
			copied = true;
			clearTimeout(timer);
			timer = setTimeout(() => (copied = false), 2000);
		} catch {
			window.prompt(m.party_share_prompt(), url);
		}
	}

	const label = $derived(copied ? m.party_copied() : m.party_copy());
</script>

<Button
	variant="ghost"
	size="icon-sm"
	class={cn(copied && 'text-emerald-600 hover:text-emerald-600 dark:text-emerald-400', className)}
	aria-label={label}
	title={label}
	onclick={copy}
	data-url={url}
	data-copied={copied || undefined}
	data-testid="party-invite"
>
	{#if copied}<CheckIcon />{:else}<LinkIcon />{/if}
</Button>
<span class="sr-only" aria-live="polite">{copied ? m.party_copied() : ''}</span>
