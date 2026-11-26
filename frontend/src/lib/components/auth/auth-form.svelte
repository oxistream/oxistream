<script lang="ts">
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import type { Snippet } from 'svelte';
	import LogoMark from '#lib/components/brand/logo-mark.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		title,
		subtitle,
		submitLabel,
		status,
		onsubmit,
		onsso,
		children,
		footer
	}: {
		title: string;
		subtitle: string;
		submitLabel: string;
		/** Announced below the submit button: an error summary or the outcome of a submit. */
		status?: string;
		onsubmit: (event: SubmitEvent) => void;
		onsso?: () => void;
		children: Snippet;
		footer: Snippet;
	} = $props();
</script>

<div class="flex w-full max-w-sm flex-col gap-6">
	<div class="flex flex-col items-center gap-3 text-center">
		<LogoMark class="size-10 text-primary" />
		<div class="flex flex-col gap-1">
			<h1 class="text-xl font-semibold tracking-tight">{title}</h1>
			<p class="text-sm text-pretty text-muted-foreground">{subtitle}</p>
		</div>
	</div>

	<div class="flex flex-col gap-5 rounded-xl border bg-background p-6 shadow-xs">
		{#if onsso}
			<Button variant="outline" class="h-10 w-full" onclick={onsso} data-testid="sso-button">
				<KeyRoundIcon />
				{m.auth_sso()}
			</Button>
			<div class="flex items-center gap-3 text-xs text-muted-foreground" aria-hidden="true">
				<span class="h-px flex-1 bg-border"></span>
				{m.auth_or()}
				<span class="h-px flex-1 bg-border"></span>
			</div>
		{/if}

		<form class="flex flex-col gap-4" novalidate {onsubmit}>
			{@render children()}
			<div class="mt-1 flex flex-col">
				<Button type="submit" class="h-10 w-full">{submitLabel}</Button>
				<p
					role="status"
					data-testid="form-status"
					class={['text-sm text-pretty text-muted-foreground', status && 'mt-3']}
				>
					{status ?? ''}
				</p>
			</div>
		</form>
	</div>

	<p class="text-center text-sm text-muted-foreground">{@render footer()}</p>
</div>
