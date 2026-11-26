<script lang="ts" module>
	export const MAX_TITLE = 150;
	export const MAX_DESCRIPTION = 5000;
</script>

<script lang="ts">
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import type { PlaylistFields, PlaylistVisibility } from '#lib/api/playlists.js';
	import SegmentedControl from '#lib/components/segmented-control.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		open = $bindable(false),
		initial,
		onsubmit
	}: {
		open?: boolean;
		/** Edits an existing playlist when set, else creates one. */
		initial?: PlaylistFields;
		/** Resolves when saved; a rejection keeps the dialog open with an error. */
		onsubmit: (fields: PlaylistFields) => Promise<unknown>;
	} = $props();

	const uid = $props.id();
	let title = $state('');
	let description = $state('');
	let visibility = $state<PlaylistVisibility>('private');
	let submitted = $state(false);
	let saving = $state(false);
	let failed = $state(false);

	// Reset from `initial` whenever the dialog opens.
	$effect.pre(() => {
		if (!open) return;
		title = initial?.title ?? '';
		description = initial?.description ?? '';
		visibility = initial?.visibility ?? 'private';
		submitted = false;
		failed = false;
	});

	const titleError = $derived.by(() => {
		const length = title.trim().length;
		if (submitted && length === 0) return m.playlist_error_title_required();
		if (length > MAX_TITLE) return m.playlist_error_too_long({ max: MAX_TITLE });
		return undefined;
	});
	const descriptionError = $derived(
		description.trim().length > MAX_DESCRIPTION
			? m.playlist_error_too_long({ max: MAX_DESCRIPTION })
			: undefined
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitted = true;
		if (title.trim().length === 0 || titleError || descriptionError) return;
		saving = true;
		failed = false;
		try {
			await onsubmit({ title: title.trim(), description: description.trim(), visibility });
			open = false;
		} catch {
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content>
		<form class="flex flex-col gap-5" onsubmit={submit} novalidate>
			<Dialog.Header>
				<Dialog.Title>{initial ? m.playlist_edit_title() : m.playlist_create_title()}</Dialog.Title>
			</Dialog.Header>
			<div class="flex flex-col gap-2">
				<Label for="{uid}-title">{m.playlist_field_title()}</Label>
				<Input
					id="{uid}-title"
					bind:value={title}
					maxlength={MAX_TITLE + 20}
					autocomplete="off"
					aria-invalid={titleError ? true : undefined}
					aria-describedby={titleError ? `${uid}-title-error` : undefined}
					class="h-10 text-base md:text-sm"
				/>
				{#if titleError}
					<p id="{uid}-title-error" class="text-xs font-medium text-destructive">{titleError}</p>
				{/if}
			</div>
			<div class="flex flex-col gap-2">
				<Label for="{uid}-description">{m.playlist_field_description()}</Label>
				<Textarea
					id="{uid}-description"
					bind:value={description}
					rows={3}
					class="max-h-48"
					aria-invalid={descriptionError ? true : undefined}
					aria-describedby={descriptionError ? `${uid}-description-error` : undefined}
				/>
				{#if descriptionError}
					<p id="{uid}-description-error" class="text-xs font-medium text-destructive">
						{descriptionError}
					</p>
				{/if}
			</div>
			<SegmentedControl
				name="{uid}-visibility"
				legend={m.playlist_field_visibility()}
				description={visibility === 'unlisted'
					? m.playlist_visibility_unlisted_hint()
					: m.playlist_visibility_private_hint()}
				options={[
					{ value: 'private', label: m.playlist_visibility_private() },
					{ value: 'unlisted', label: m.playlist_visibility_unlisted() }
				]}
				value={visibility}
				onchange={(value) => (visibility = value)}
			/>
			{#if failed}
				<p class="text-sm font-medium text-destructive" role="alert">{m.playlist_error_save()}</p>
			{/if}
			<Dialog.Footer>
				<Button variant="outline" type="button" onclick={() => (open = false)}>
					{m.action_cancel()}
				</Button>
				<Button type="submit" disabled={saving} aria-busy={saving || undefined}>
					{#if saving}<LoaderCircleIcon class="animate-spin motion-reduce:animate-none" />{/if}
					{initial ? m.action_save() : m.playlist_create_submit()}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
