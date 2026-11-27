<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import LinkIcon from '@lucide/svelte/icons/link';
	import QrCodeIcon from '@lucide/svelte/icons/qr-code';
	import ShareIcon from '@lucide/svelte/icons/share';
	import { onMount } from 'svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import QrCode from './qr-code.svelte';

	let { url, class: className }: { url: string; class?: string } = $props();

	let copied = $state(false);
	let qrOpen = $state(false);
	let canShare = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	onMount(() => {
		canShare = typeof navigator.share === 'function';
		return () => clearTimeout(timer);
	});

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

	async function share() {
		try {
			await navigator.share({ title: m.party_title(), url });
		} catch {
			// Dismissing the share sheet rejects as well.
		}
	}

	const label = $derived(copied ? m.party_copied() : m.party_invite());
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button
				{...props}
				variant="ghost"
				size="icon-sm"
				class={cn(
					copied && 'text-emerald-600 hover:text-emerald-600 dark:text-emerald-400',
					className
				)}
				aria-label={label}
				title={label}
				data-url={url}
				data-copied={copied || undefined}
				data-testid="party-invite"
			>
				{#if copied}<CheckIcon />{:else}<LinkIcon />{/if}
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end" class="w-auto min-w-40 whitespace-nowrap">
		<DropdownMenu.Item onSelect={copy} data-testid="party-invite-copy">
			<CopyIcon />
			{m.party_copy()}
		</DropdownMenu.Item>
		<DropdownMenu.Item onSelect={() => (qrOpen = true)} data-testid="party-invite-qr">
			<QrCodeIcon />
			{m.party_qr()}
		</DropdownMenu.Item>
		{#if canShare}
			<DropdownMenu.Item onSelect={share} data-testid="party-invite-share">
				<ShareIcon />
				{m.party_share()}
			</DropdownMenu.Item>
		{/if}
	</DropdownMenu.Content>
</DropdownMenu.Root>
<span class="sr-only" aria-live="polite">{copied ? m.party_copied() : ''}</span>

<Dialog.Root bind:open={qrOpen}>
	<Dialog.Content class="gap-4 sm:max-w-xs">
		<Dialog.Header>
			<Dialog.Title>{m.party_qr_title()}</Dialog.Title>
		</Dialog.Header>
		<QrCode value={url} label={m.party_qr()} class="mx-auto max-w-72" />
		<p class="truncate text-center font-mono text-xs text-muted-foreground" title={url}>
			{url}
		</p>
	</Dialog.Content>
</Dialog.Root>
