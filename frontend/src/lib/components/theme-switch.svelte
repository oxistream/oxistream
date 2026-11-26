<script lang="ts">
	import MoonIcon from '@lucide/svelte/icons/moon';
	import SunIcon from '@lucide/svelte/icons/sun';
	import { setMode, userPrefersMode } from 'mode-watcher';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';

	type Mode = 'light' | 'dark' | 'system';

	const modes: { value: Mode; label: () => string }[] = [
		{ value: 'light', label: m.theme_light },
		{ value: 'dark', label: m.theme_dark },
		{ value: 'system', label: m.theme_system }
	];
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class={buttonVariants({ variant: 'ghost', size: 'icon' })}
		aria-label={m.theme_label()}
	>
		<SunIcon class="scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90" />
		<MoonIcon class="absolute scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0" />
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end">
		<DropdownMenu.Label>{m.theme_label()}</DropdownMenu.Label>
		<DropdownMenu.RadioGroup
			value={userPrefersMode.current}
			onValueChange={(value) => setMode(value as Mode)}
		>
			{#each modes as mode (mode.value)}
				<DropdownMenu.RadioItem value={mode.value}>{mode.label()}</DropdownMenu.RadioItem>
			{/each}
		</DropdownMenu.RadioGroup>
	</DropdownMenu.Content>
</DropdownMenu.Root>
