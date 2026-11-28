<script lang="ts">
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import { ApiError } from '#lib/api/mapping.js';
	import {
		isNameTaken,
		MAX_GROUP_NAME,
		MAX_GROUPS,
		type SubscriptionGroup
	} from '#lib/api/subscription-group-list.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';

	let {
		groups,
		initial = '',
		exceptId,
		label,
		submitLabel,
		autofocus = false,
		onsubmit,
		oncancel,
		class: className
	}: {
		groups: SubscriptionGroup[];
		initial?: string;
		/** The group being renamed, whose own name is not taken. */
		exceptId?: string;
		label: string;
		submitLabel: string;
		autofocus?: boolean;
		/** Resolves when saved; a rejection shows an error. */
		onsubmit: (name: string) => Promise<unknown>;
		oncancel?: () => void;
		class?: string;
	} = $props();

	const uid = $props.id();
	// svelte-ignore state_referenced_locally
	let name = $state(initial);
	let submitted = $state(false);
	let saving = $state(false);
	let failure = $state<string>();

	const trimmed = $derived(name.trim());
	const error = $derived.by(() => {
		if (trimmed.length > MAX_GROUP_NAME) return m.groups_error_too_long({ max: MAX_GROUP_NAME });
		if (trimmed && isNameTaken(groups, trimmed, exceptId)) return m.groups_error_taken();
		if (submitted && !trimmed) return m.groups_error_required();
		return failure;
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitted = true;
		failure = undefined;
		if (!trimmed || error) return;
		if (trimmed === initial.trim() && exceptId) {
			oncancel?.();
			return;
		}
		saving = true;
		try {
			await onsubmit(trimmed);
			name = '';
			submitted = false;
		} catch (err) {
			failure =
				err instanceof ApiError && err.status === 422
					? m.groups_error_taken()
					: err instanceof ApiError && err.code === 'limit_reached'
						? m.groups_error_limit({ max: MAX_GROUPS })
						: m.video_action_failed();
		} finally {
			saving = false;
		}
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && oncancel) {
			event.preventDefault();
			event.stopPropagation();
			oncancel();
		}
	}
</script>

<form class={cn('flex flex-col gap-1.5', className)} onsubmit={submit} novalidate>
	<div class="flex gap-2">
		<Input
			id="{uid}-name"
			bind:value={name}
			oninput={() => (failure = undefined)}
			onkeydown={keydown}
			aria-label={label}
			placeholder={label}
			autocomplete="off"
			{autofocus}
			maxlength={MAX_GROUP_NAME + 10}
			aria-invalid={error ? true : undefined}
			aria-describedby={error ? `${uid}-error` : undefined}
			class="h-9 min-w-0 flex-1 text-base md:text-sm"
			data-testid="group-name-input"
		/>
		<Button
			type="submit"
			size="sm"
			class="h-9 shrink-0"
			disabled={saving}
			aria-busy={saving || undefined}
			data-testid="group-name-submit"
		>
			{#if saving}<LoaderCircleIcon class="animate-spin motion-reduce:animate-none" />{/if}
			{submitLabel}
		</Button>
	</div>
	{#if error}
		<p id="{uid}-error" class="text-xs font-medium text-destructive" role="alert">{error}</p>
	{/if}
</form>
