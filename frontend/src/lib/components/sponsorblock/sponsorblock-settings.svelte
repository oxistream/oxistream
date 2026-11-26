<script lang="ts">
	import '#lib/sponsorblock/colors.css';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { categoryColor, categoryHint, categoryLabel } from '#lib/sponsorblock/categories.js';
	import {
		SKIP_CATEGORIES,
		type SkipCategory,
		type SkipMode,
		type SponsorBlockSettings
	} from '#lib/sponsorblock/settings.js';
	import { cn } from '#lib/utils.js';

	let {
		settings,
		onchange,
		signedIn = false,
		saveFailed = false,
		headingLevel = 2,
		header = true,
		class: className
	}: {
		settings: SponsorBlockSettings;
		onchange: (next: SponsorBlockSettings) => void;
		/** Guests are told their choice stays in this browser. */
		signedIn?: boolean;
		saveFailed?: boolean;
		headingLevel?: 2 | 3;
		/** Off when the surrounding dialog shows title and description itself. */
		header?: boolean;
		class?: string;
	} = $props();

	const uid = $props.id();
	const modes: { value: SkipMode; label: () => string }[] = [
		{ value: 'auto', label: m.sponsorblock_mode_auto },
		{ value: 'button', label: m.sponsorblock_mode_button },
		{ value: 'off', label: m.sponsorblock_mode_off }
	];

	const toggles = [
		{
			key: 'highlight',
			label: m.sponsorblock_highlight_toggle,
			hint: m.sponsorblock_highlight_hint,
			category: 'poi_highlight'
		},
		{
			key: 'full_video_label',
			label: m.sponsorblock_full_video_toggle,
			hint: m.sponsorblock_full_video_hint,
			category: 'exclusive_access'
		}
	] as const;

	function setMode(category: SkipCategory, mode: SkipMode) {
		onchange({ ...settings, categories: { ...settings.categories, [category]: mode } });
	}
</script>

<section
	aria-labelledby={header ? `${uid}-heading` : undefined}
	aria-label={header ? undefined : m.sponsorblock_title()}
	class={cn('flex flex-col gap-5', className)}
>
	{#if header || !signedIn || saveFailed}
		<div class="flex flex-col gap-1">
			{#if header}
				<svelte:element
					this={`h${headingLevel}`}
					id="{uid}-heading"
					class="text-base font-semibold tracking-tight"
				>
					{m.sponsorblock_title()}
				</svelte:element>
				<p class="text-sm text-pretty text-muted-foreground">{m.sponsorblock_description()}</p>
			{/if}
			{#if !signedIn}
				<p class="text-sm text-muted-foreground">{m.sponsorblock_saved_locally()}</p>
			{/if}
			{#if saveFailed}
				<p role="alert" class="text-sm text-destructive">{m.sponsorblock_save_failed()}</p>
			{/if}
		</div>
	{/if}

	<ul class="flex flex-col divide-y" aria-label={m.sponsorblock_categories()}>
		{#each SKIP_CATEGORIES as category (category)}
			{@const name = `${uid}-${category}`}
			<li class="flex flex-col gap-2 py-3 first:pt-0 sm:flex-row sm:items-center sm:gap-4">
				<div class="flex min-w-0 flex-1 items-start gap-2.5">
					<span
						aria-hidden="true"
						class="mt-1.5 size-2.5 shrink-0 rounded-full"
						style:background-color={categoryColor(category)}
					></span>
					<div class="min-w-0">
						<p id="{name}-label" class="text-sm font-medium">{categoryLabel(category)}</p>
						<p id="{name}-hint" class="text-sm text-muted-foreground">{categoryHint(category)}</p>
					</div>
				</div>
				<div
					role="radiogroup"
					aria-labelledby="{name}-label"
					aria-describedby="{name}-hint"
					data-category={category}
					class="ml-5 inline-flex w-fit shrink-0 gap-1 rounded-lg bg-muted p-1 sm:ml-0"
				>
					{#each modes as mode (mode.value)}
						<label
							class="relative inline-flex h-7 cursor-pointer items-center rounded-md px-2.5 text-sm font-medium text-muted-foreground transition-colors duration-(--motion-in) ease-ui hover:text-foreground has-checked:bg-background has-checked:text-foreground has-checked:shadow-xs has-focus-visible:outline-2 has-focus-visible:outline-offset-2 has-focus-visible:outline-ring has-focus-visible:outline-solid"
						>
							<input
								type="radio"
								{name}
								value={mode.value}
								checked={settings.categories[category] === mode.value}
								onchange={() => setMode(category, mode.value)}
								class="sr-only"
							/>
							{mode.label()}
						</label>
					{/each}
				</div>
			</li>
		{/each}
	</ul>

	<div class="flex flex-col gap-4">
		{#each toggles as toggle (toggle.key)}
			{@const id = `${uid}-${toggle.key}`}
			<div class="flex items-start gap-3">
				<Checkbox
					{id}
					class="mt-0.5"
					aria-describedby="{id}-hint"
					checked={settings[toggle.key]}
					onCheckedChange={(checked) => onchange({ ...settings, [toggle.key]: checked })}
				/>
				<div class="min-w-0">
					<label for={id} class="flex cursor-pointer items-center gap-2 text-sm font-medium">
						{toggle.label()}
						<span
							aria-hidden="true"
							class="size-2.5 rounded-full"
							style:background-color={categoryColor(toggle.category)}
						></span>
					</label>
					<p id="{id}-hint" class="text-sm text-muted-foreground">{toggle.hint()}</p>
				</div>
			</div>
		{/each}
	</div>
</section>
