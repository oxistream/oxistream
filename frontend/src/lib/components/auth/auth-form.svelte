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
		busy = false,
		local = true,
		onsubmit,
		sso,
		children,
		footer
	}: {
		title: string;
		subtitle: string;
		submitLabel: string;
		/** Announced below the submit button: an error summary or the outcome of a submit. */
		status?: string;
		/** Disables the submit button while a request is in flight. */
		busy?: boolean;
		/** Shows the username and password form; off when only SSO is offered. */
		local?: boolean;
		onsubmit: (event: SubmitEvent) => void;
		/** Single sign-on button, shown when the instance offers it. */
		sso?: { label: string; href: string };
		children: Snippet;
		footer?: Snippet;
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
		{#if sso}
			<!-- A full page load: the API redirects to the provider. -->
			<Button
				variant="outline"
				class="h-10 w-full"
				href={sso.href}
				data-sveltekit-reload
				data-testid="sso-button"
			>
				<KeyRoundIcon />
				{sso.label}
			</Button>
			{#if local}
				<div class="flex items-center gap-3 text-xs text-muted-foreground" aria-hidden="true">
					<span class="h-px flex-1 bg-border"></span>
					{m.auth_or()}
					<span class="h-px flex-1 bg-border"></span>
				</div>
			{/if}
		{/if}

		{#if local}
			<form class="flex flex-col gap-4" novalidate {onsubmit} aria-busy={busy}>
				{@render children()}
				<div class="mt-1 flex flex-col">
					<Button type="submit" class="h-10 w-full" disabled={busy}>{submitLabel}</Button>
					<p
						role="status"
						data-testid="form-status"
						class={['text-sm text-pretty text-muted-foreground', status && 'mt-3']}
					>
						{status ?? ''}
					</p>
				</div>
			</form>
		{:else if status || !sso}
			<p role="status" data-testid="form-status" class="text-sm text-pretty text-muted-foreground">
				{status || m.auth_local_disabled()}
			</p>
		{/if}
	</div>

	{#if footer}
		<p class="text-center text-sm text-muted-foreground">{@render footer()}</p>
	{/if}
</div>
