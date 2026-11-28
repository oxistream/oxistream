<script lang="ts">
	import SendHorizontalIcon from '@lucide/svelte/icons/send-horizontal';
	import { tick } from 'svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { formatNumber } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { REACTIONS, continues } from '#lib/together/chat.js';
	import {
		MAX_CHAT_LENGTH,
		type RoomChatMessage,
		type RoomReaction
	} from '#lib/together/connection.svelte.js';
	import { cn } from '#lib/utils.js';

	/** Within this distance of the bottom, new messages keep the chat scrolled down. */
	const STICK_PX = 80;
	/** The character count shows this close to the limit. */
	const COUNT_FROM = 50;

	let {
		messages,
		disabled = false,
		onsend,
		onreact,
		class: className
	}: {
		/** Oldest first. */
		messages: RoomChatMessage[];
		/** E.g. while not connected. */
		disabled?: boolean;
		onsend: (text: string) => void;
		onreact: (reaction: RoomReaction) => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	const clock = new Intl.DateTimeFormat(locale, { hour: 'numeric', minute: '2-digit' });
	let root = $state<HTMLDivElement>();
	let text = $state('');
	let stick = true;
	const remaining = $derived(MAX_CHAT_LENGTH - [...text].length);

	/** The element that scrolls the chat: the panel body. */
	function scroller(): HTMLElement | null {
		let node = root?.parentElement ?? null;
		while (node) {
			if (/(auto|scroll)/.test(getComputedStyle(node).overflowY)) return node;
			node = node.parentElement;
		}
		return null;
	}

	$effect.pre(() => {
		void messages.length;
		const area = scroller();
		const own = messages.at(-1)?.own ?? false;
		stick = own || !area || area.scrollHeight - area.scrollTop - area.clientHeight < STICK_PX;
	});

	$effect(() => {
		void messages.length;
		if (!stick) return;
		void tick().then(() => {
			const area = scroller();
			if (area) area.scrollTop = area.scrollHeight;
		});
	});

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const message = text.trim();
		if (!message || disabled) return;
		onsend(message);
		text = '';
	}
</script>

<div bind:this={root} class={cn('flex flex-col', className)} data-testid="party-chat">
	{#if messages.length === 0}
		<p class="px-2 py-1.5 text-sm text-muted-foreground">{m.party_chat_empty()}</p>
	{:else}
		<ol class="flex flex-col px-2 pb-1" aria-live="polite" aria-relevant="additions">
			{#each messages as message, index (message.id)}
				{@const joined = continues(messages[index - 1], message)}
				<li
					class={cn('flex min-w-0 flex-col', !joined && index > 0 && 'mt-2.5')}
					data-testid="party-chat-message"
					data-own={message.own ? '' : undefined}
				>
					{#if !joined}
						<p class="flex min-w-0 items-baseline gap-1.5 text-xs leading-5">
							<span
								class={cn('truncate font-medium', message.own ? 'text-primary' : 'text-foreground')}
							>
								{message.name}
							</span>
							{#if message.is_owner}
								<span class="shrink-0 text-[0.6875rem] font-medium text-primary">
									{m.party_host()}
								</span>
							{/if}
							<time
								class="shrink-0 text-muted-foreground tabular-nums"
								datetime={new Date(message.sent_at).toISOString()}
							>
								{clock.format(message.sent_at)}
							</time>
						</p>
					{/if}
					<p class="text-sm leading-5 text-pretty [overflow-wrap:anywhere]">{message.text}</p>
				</li>
			{/each}
		</ol>
	{/if}
	<div class="sticky bottom-0 z-10 flex flex-col gap-1 bg-sidebar px-1 pt-1.5 pb-0.5">
		<div class="flex items-center gap-0.5" role="group" aria-label={m.party_reactions()}>
			{#each REACTIONS as reaction (reaction.id)}
				<Button
					variant="ghost"
					size="icon-sm"
					class="text-base"
					aria-label={reaction.label()}
					title={reaction.label()}
					{disabled}
					onclick={() => onreact(reaction.id)}
					data-testid="party-react-{reaction.id}"
				>
					<span aria-hidden="true">{reaction.emoji}</span>
				</Button>
			{/each}
		</div>
		<form class="flex items-center gap-1" onsubmit={submit}>
			<Input
				bind:value={text}
				class="h-8"
				maxlength={MAX_CHAT_LENGTH}
				placeholder={m.party_chat_placeholder()}
				aria-label={m.party_chat_label()}
				autocomplete="off"
				enterkeyhint="send"
				data-testid="party-chat-input"
			/>
			<Button
				type="submit"
				size="icon-sm"
				class="shrink-0"
				disabled={disabled || !text.trim()}
				aria-label={m.party_chat_send()}
				title={m.party_chat_send()}
				data-testid="party-chat-send"
			>
				<SendHorizontalIcon />
			</Button>
		</form>
		{#if remaining <= COUNT_FROM}
			<p class="px-1 text-end text-xs text-muted-foreground tabular-nums" aria-live="polite">
				{formatNumber(remaining, locale)}
			</p>
		{/if}
	</div>
</div>
