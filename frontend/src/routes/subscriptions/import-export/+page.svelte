<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import FileUpIcon from '@lucide/svelte/icons/file-up';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import TvMinimalPlayIcon from '@lucide/svelte/icons/tv-minimal-play';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { resolve } from '$app/paths';
	import {
		exportHref,
		formatLabel,
		importSubscriptions,
		type ExportFormat,
		type ImportResult
	} from '#lib/api/subscription-files.js';
	import { subscriptionKeys } from '#lib/api/subscriptions.js';
	import PageHeader from '#lib/components/page-header.svelte';
	import SignInActions from '#lib/components/sign-in-actions.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';

	type ImportState =
		| { kind: 'idle' }
		| { kind: 'uploading'; name: string }
		| { kind: 'done'; name: string; result: ImportResult }
		| { kind: 'failed'; name: string; message: string };

	const queryClient = useQueryClient();

	let status = $state<ImportState>({ kind: 'idle' });
	let dragging = $state(false);
	let input = $state<HTMLInputElement | null>(null);

	const exports: { format: ExportFormat; title: string; body?: string }[] = [
		{ format: 'newpipe', title: m.export_newpipe_title(), body: m.export_newpipe_body() },
		{ format: 'invidious', title: m.export_invidious_title() }
	];

	const number = $derived(new Intl.NumberFormat(getLocale()));

	async function upload(file: File | undefined) {
		if (!file || status.kind === 'uploading') return;
		status = { kind: 'uploading', name: file.name };
		const outcome = await importSubscriptions(file);
		if (outcome.ok) {
			status = { kind: 'done', name: file.name, result: outcome.result };
			if (outcome.result.imported > 0) {
				void queryClient.invalidateQueries({ queryKey: subscriptionKeys.all });
				void queryClient.invalidateQueries({ queryKey: subscriptionKeys.feed });
			}
		} else {
			status = { kind: 'failed', name: file.name, message: outcome.message };
		}
		if (input) input.value = '';
	}

	function ondragover(event: DragEvent) {
		if (!event.dataTransfer?.types.includes('Files')) return;
		event.preventDefault();
		event.dataTransfer.dropEffect = 'copy';
		dragging = true;
	}

	function ondragleave(event: DragEvent) {
		const target = event.currentTarget as HTMLElement;
		if (!target.contains(event.relatedTarget as Node | null)) dragging = false;
	}

	function ondrop(event: DragEvent) {
		event.preventDefault();
		dragging = false;
		void upload(event.dataTransfer?.files[0]);
	}

	function counts(result: ImportResult) {
		return [
			{ label: m.import_count_imported(), value: result.imported, accent: result.imported > 0 },
			{ label: m.import_count_existing(), value: result.already_subscribed, accent: false },
			{ label: m.import_count_skipped(), value: result.skipped, accent: false },
			{ label: m.import_count_invalid(), value: result.invalid, accent: false }
		];
	}
</script>

<div class="page-column flex flex-col gap-6 py-6">
	<PageHeader title={m.import_export_title()} />

	{#if !session.ready}
		<div class="h-72 animate-pulse rounded-xl border bg-card"></div>
	{:else if !session.user}
		<EmptyState icon={TvMinimalPlayIcon} title={m.import_export_sign_in_title()}>
			{#snippet actions()}<SignInActions />{/snippet}
		</EmptyState>
	{:else}
		<section
			aria-labelledby="import-heading"
			class="flex flex-col gap-4 rounded-xl border bg-card p-5"
		>
			<div class="flex flex-col gap-1">
				<h2 id="import-heading" class="text-base font-semibold tracking-tight">
					{m.import_heading()}
				</h2>
				<p class="text-sm text-pretty text-muted-foreground">{m.import_hint()}</p>
			</div>

			{#if status.kind === 'done'}
				{@const result = status.result}
				<div class="flex flex-col gap-4" aria-live="polite">
					<div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
						<span class="text-muted-foreground">{m.import_detected()}</span>
						<span
							class="rounded-md bg-primary/10 px-2 py-0.5 font-medium text-primary"
							data-testid="import-format"
						>
							{formatLabel(result.format)}
						</span>
						<span class="min-w-0 truncate text-muted-foreground">· {status.name}</span>
					</div>
					<dl class="grid grid-cols-2 gap-2 sm:grid-cols-4">
						{#each counts(result) as count (count.label)}
							<div class="flex flex-col gap-0.5 rounded-lg border bg-background px-3 py-2.5">
								<dt class="text-xs text-muted-foreground">{count.label}</dt>
								<dd
									class={cn(
										'text-xl font-semibold tracking-tight tabular-nums',
										count.accent && 'text-primary'
									)}
								>
									{number.format(count.value)}
								</dd>
							</div>
						{/each}
					</dl>
					{#if result.skipped > 0}
						<details class="group rounded-lg border bg-background text-sm">
							<summary
								class="cursor-pointer rounded-lg px-3 py-2.5 font-medium select-none hover:bg-muted"
							>
								{m.import_skipped_show({ count: number.format(result.skipped) })}
							</summary>
							<div class="flex flex-col gap-2 border-t px-3 py-2.5">
								<p class="text-muted-foreground">{m.import_skipped_hint()}</p>
								<ul class="flex max-h-48 flex-col gap-1 overflow-y-auto font-mono text-xs">
									{#each result.skipped_entries as entry, i (i)}
										<li class="truncate" title={entry}>{entry}</li>
									{/each}
								</ul>
								{#if result.skipped > result.skipped_entries.length}
									<p class="text-xs text-muted-foreground">
										{m.import_skipped_more({
											count: number.format(result.skipped - result.skipped_entries.length)
										})}
									</p>
								{/if}
							</div>
						</details>
					{/if}
					{#if result.imported > 0}
						<p class="text-sm text-muted-foreground">{m.import_pending_hint()}</p>
					{/if}
					<div class="flex flex-wrap gap-2">
						<Button href={resolve('/feed/subscriptions')}>
							{m.import_open_subscriptions()}
							<ArrowRightIcon data-icon="inline-end" />
						</Button>
						<Button variant="outline" onclick={() => (status = { kind: 'idle' })}>
							{m.import_another()}
						</Button>
					</div>
				</div>
			{:else}
				<label
					class={cn(
						'group relative flex cursor-pointer flex-col items-center justify-center gap-3 rounded-lg border border-dashed px-6 py-10 text-center transition-colors duration-(--motion-in) ease-ui',
						'focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ring hover:border-primary/50 hover:bg-muted/50',
						dragging && 'border-primary bg-primary/5',
						status.kind === 'uploading' && 'pointer-events-none opacity-70'
					)}
					{ondragover}
					ondragenter={ondragover}
					{ondragleave}
					{ondrop}
				>
					<input
						bind:this={input}
						type="file"
						class="sr-only"
						accept=".csv,.json,.opml,.xml,text/csv,application/json,text/xml,application/xml"
						aria-label={m.import_drop_label()}
						disabled={status.kind === 'uploading'}
						onchange={(event) => void upload(event.currentTarget.files?.[0])}
					/>
					<div
						class="flex size-11 items-center justify-center rounded-full bg-muted text-muted-foreground"
					>
						{#if status.kind === 'uploading'}
							<LoaderCircleIcon class="size-5 animate-spin" />
						{:else}
							<FileUpIcon class="size-5" />
						{/if}
					</div>
					{#if status.kind === 'uploading'}
						<p class="text-sm font-medium" aria-live="polite">
							{m.import_uploading({ name: status.name })}
						</p>
					{:else}
						<div class="flex flex-col gap-0.5">
							<p class="text-sm font-medium">{m.import_drop_title()}</p>
							<p class="text-sm">
								<span class="font-medium text-primary underline-offset-4 group-hover:underline">
									{m.import_drop_choose()}
								</span>
								<span class="text-muted-foreground"> · {m.import_drop_limit()}</span>
							</p>
						</div>
					{/if}
				</label>
				{#if status.kind === 'failed'}
					<div
						role="alert"
						class="flex gap-3 rounded-lg border border-destructive/25 bg-destructive/5 px-3 py-2.5 text-sm"
					>
						<CircleAlertIcon class="mt-0.5 size-4 shrink-0 text-destructive" />
						<div class="flex min-w-0 flex-col gap-0.5">
							<p class="font-medium">{m.import_error_title()}</p>
							<p class="text-muted-foreground">{status.message}</p>
							<p class="truncate text-xs text-muted-foreground">{status.name}</p>
						</div>
					</div>
				{/if}
			{/if}
		</section>

		<section
			aria-labelledby="export-heading"
			class="flex flex-col gap-4 rounded-xl border bg-card p-5"
		>
			<h2 id="export-heading" class="text-base font-semibold tracking-tight">
				{m.export_heading()}
			</h2>
			<ul class="flex flex-col divide-y rounded-lg border bg-background">
				{#each exports as item (item.format)}
					<li class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-3 py-3">
						<div class="flex min-w-0 flex-col gap-0.5">
							<p class="text-sm font-medium">{item.title}</p>
							{#if item.body}
								<p class="text-sm text-muted-foreground">{item.body}</p>
							{/if}
						</div>
						<Button
							variant="outline"
							href={exportHref(item.format)}
							download
							aria-label={`${m.export_download()}: ${item.title}`}
						>
							<DownloadIcon data-icon="inline-start" />
							{m.export_download()}
						</Button>
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</div>
