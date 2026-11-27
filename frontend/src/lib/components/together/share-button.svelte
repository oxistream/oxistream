<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import LinkIcon from '@lucide/svelte/icons/link';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';

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
</script>

<Button variant="outline" size="sm" class={className} onclick={copy} data-testid="party-share">
	{#if copied}<CheckIcon />{:else}<LinkIcon />{/if}
	{copied ? m.party_copied() : m.party_copy()}
</Button>
<span class="sr-only" aria-live="polite">{copied ? m.party_copied() : ''}</span>
