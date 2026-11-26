<script lang="ts">
	import LanguagesIcon from '@lucide/svelte/icons/languages';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';

	const labels: Record<Locale, () => string> = {
		en: m.language_en,
		de: m.language_de
	};
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class={buttonVariants({ variant: 'ghost', size: 'icon' })}
		aria-label={m.language_label()}
		data-testid="language-switch"
	>
		<LanguagesIcon />
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end">
		<DropdownMenu.Label>{m.language_label()}</DropdownMenu.Label>
		<DropdownMenu.RadioGroup
			value={getLocale()}
			onValueChange={(value) => setLocale(value as Locale)}
		>
			{#each locales as locale (locale)}
				<DropdownMenu.RadioItem value={locale} lang={locale}>
					{labels[locale]()}
				</DropdownMenu.RadioItem>
			{/each}
		</DropdownMenu.RadioGroup>
	</DropdownMenu.Content>
</DropdownMenu.Root>
