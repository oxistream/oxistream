<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import RssIcon from '@lucide/svelte/icons/rss';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import SettingRow from './setting-row.svelte';

	let {
		createdAt,
		url = null,
		disabled = false,
		oncreate,
		ondelete
	}: {
		/** When the current link was created; `null` without one. */
		createdAt: string | null;
		/** The link, only known right after creating it. */
		url?: string | null;
		disabled?: boolean;
		oncreate: () => void;
		ondelete: () => void;
	} = $props();

	let confirming = $state<'regenerate' | 'delete' | null>(null);
	let copied = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	const description = $derived.by(() => {
		if (url) return m.feed_token_shown_once();
		if (!createdAt) return m.feed_token_none();
		const date = new Intl.DateTimeFormat(getLocale(), { dateStyle: 'long' }).format(
			new Date(createdAt)
		);
		return m.feed_token_active({ date });
	});

	async function copy() {
		if (!url) return;
		try {
			await navigator.clipboard.writeText(url);
			copied = true;
			clearTimeout(timer);
			timer = setTimeout(() => (copied = false), 2000);
		} catch {
			window.prompt(m.feed_token_url(), url);
		}
	}

	function confirm() {
		const action = confirming;
		confirming = null;
		if (action === 'regenerate') oncreate();
		else if (action === 'delete') ondelete();
	}
</script>

<SettingRow id={url ? 'settings-feed-url' : undefined} label={m.feed_token_title()} {description}>
	{#snippet control()}
		{#if createdAt}
			<div class="flex flex-wrap gap-2">
				<Button variant="outline" size="sm" {disabled} onclick={() => (confirming = 'regenerate')}>
					<RefreshCwIcon />
					{m.feed_token_regenerate()}
				</Button>
				<Button variant="outline" size="sm" {disabled} onclick={() => (confirming = 'delete')}>
					<Trash2Icon />
					{m.feed_token_delete()}
				</Button>
			</div>
		{:else}
			<Button variant="outline" size="sm" {disabled} onclick={oncreate}>
				<RssIcon />
				{m.feed_token_create()}
			</Button>
		{/if}
	{/snippet}
	{#if url}
		<div class="flex gap-2">
			<Input
				id="settings-feed-url"
				readonly
				value={url}
				class="font-mono text-xs md:text-xs"
				aria-describedby="settings-feed-url-description"
				onfocus={(event) => event.currentTarget.select()}
			/>
			<Button variant="secondary" size="sm" class="h-9 shrink-0" onclick={copy}>
				{#if copied}<CheckIcon />{:else}<CopyIcon />{/if}
				{copied ? m.feed_token_copied() : m.feed_token_copy()}
			</Button>
			<span class="sr-only" aria-live="polite">{copied ? m.feed_token_copied() : ''}</span>
		</div>
	{/if}
</SettingRow>

<AlertDialog.Root
	open={confirming !== null}
	onOpenChange={(open) => {
		if (!open) confirming = null;
	}}
>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>
				{confirming === 'delete' ? m.feed_token_delete_title() : m.feed_token_regenerate_title()}
			</AlertDialog.Title>
			<AlertDialog.Description>{m.feed_token_revoke_body()}</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
			<AlertDialog.Action variant="destructive" onclick={confirm}>
				{confirming === 'delete' ? m.feed_token_delete() : m.feed_token_regenerate()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
