<script lang="ts">
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { SHORTCUT_GROUPS, type ShortcutAction, type ShortcutKey } from '#lib/player/shortcuts.js';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	const groupLabels = {
		playback: m.shortcuts_group_playback,
		sound: m.shortcuts_group_sound,
		view: m.shortcuts_group_view
	};

	const actionLabels: Record<ShortcutAction, () => string> = {
		play: m.shortcut_play,
		back10: m.shortcut_back10,
		forward10: m.shortcut_forward10,
		back5: m.shortcut_back5,
		forward5: m.shortcut_forward5,
		percent: m.shortcut_percent,
		start: m.shortcut_start,
		end: m.shortcut_end,
		slower: m.shortcut_slower,
		faster: m.shortcut_faster,
		previous: m.shortcut_previous,
		next: m.shortcut_next,
		volumeUp: m.shortcut_volumeUp,
		volumeDown: m.shortcut_volumeDown,
		mute: m.shortcut_mute,
		fullscreen: m.shortcut_fullscreen,
		theater: m.shortcut_theater,
		captions: m.shortcut_captions,
		pip: m.shortcut_pip,
		help: m.shortcut_help
	};

	const keyNames = {
		space: m.key_space,
		home: m.key_home,
		end: m.key_end,
		shift: m.key_shift
	};

	function keyLabel(key: ShortcutKey) {
		return 'text' in key ? key.text : keyNames[key.name]();
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content
		class="max-h-[min(40rem,calc(100svh-2rem))] overflow-y-auto sm:max-w-[min(42rem,calc(100%-2rem))]"
		data-testid="shortcuts-dialog"
	>
		<Dialog.Header>
			<Dialog.Title>{m.shortcuts_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="min-w-0 gap-x-8 sm:columns-2">
			{#each SHORTCUT_GROUPS as group (group.id)}
				<section class="mb-5 break-inside-avoid last:mb-0">
					<h3 class="mb-1.5 text-xs font-medium text-muted-foreground">
						{groupLabels[group.id]()}
					</h3>
					<dl class="grid gap-0.5">
						{#each group.entries as entry (entry.action)}
							<div class="flex min-h-8 items-center justify-between gap-4">
								<dt class="min-w-0 truncate">{actionLabels[entry.action]()}</dt>
								<dd class="flex shrink-0 items-center gap-1.5">
									{#each entry.keys as combo, i (i)}
										<span class="flex items-center gap-1">
											{#each combo as key, j (j)}
												<kbd
													class="inline-flex h-6 min-w-6 items-center justify-center rounded border bg-muted px-1.5 font-sans text-xs whitespace-nowrap text-foreground"
												>
													{keyLabel(key)}
												</kbd>
											{/each}
										</span>
									{/each}
								</dd>
							</div>
						{/each}
					</dl>
				</section>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>
