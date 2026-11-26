<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { readJson, writeJson } from '#lib/components/player/storage.js';
	import * as Collapsible from '#lib/components/ui/collapsible/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { isActive } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { sectionHref, visibleSections } from '#lib/settings/sections.js';
	import { cn } from '#lib/utils.js';

	let { signedIn, class: className }: { signedIn: boolean; class?: string } = $props();

	const STORAGE_KEY = 'oxistream:nav-settings-open';
	const href = resolve('/settings');

	const sections = $derived(
		visibleSections(signedIn).map((section) => ({ section, href: sectionHref(section) }))
	);
	const onSettings = $derived(isActive(page.url.pathname, href));

	let remembered = readJson(STORAGE_KEY, (v): v is boolean => typeof v === 'boolean') ?? false;
	// Opens on settings routes; elsewhere it is as the user last left it.
	let open = $derived(onSettings || remembered);

	function toggle(value: boolean) {
		open = value;
		remembered = value;
		writeJson(STORAGE_KEY, value);
	}
</script>

{#snippet tooltip()}
	<div class="flex flex-col gap-1 py-0.5">
		<span class="font-medium">{m.nav_settings()}</span>
		<ul class="flex flex-col gap-0.5 opacity-75">
			{#each sections as { section } (section.id)}
				<li>{section.title()}</li>
			{/each}
		</ul>
	</div>
{/snippet}

<Sidebar.Group class={cn(className)}>
	<Sidebar.Menu>
		<Collapsible.Root {open} onOpenChange={toggle}>
			{#snippet child({ props })}
				<Sidebar.MenuItem {...props}>
					<Sidebar.MenuButton isActive={onSettings} tooltipContent={tooltip}>
						{#snippet child({ props })}
							<a
								{href}
								aria-current={page.url.pathname === href ? 'page' : undefined}
								{...props}
								class={cn(props.class as string, 'data-active:[&>svg]:text-sidebar-primary')}
							>
								<SettingsIcon />
								<span>{m.nav_settings()}</span>
							</a>
						{/snippet}
					</Sidebar.MenuButton>
					<Collapsible.Trigger>
						{#snippet child({ props })}
							<Sidebar.MenuAction
								{...props}
								class="transition-transform duration-(--motion-in) ease-ui data-[state=open]:rotate-90"
							>
								<ChevronRightIcon />
								<span class="sr-only">{m.settings_toggle_sections()}</span>
							</Sidebar.MenuAction>
						{/snippet}
					</Collapsible.Trigger>
					<Collapsible.Content>
						<Sidebar.MenuSub aria-label={m.settings_sections()}>
							{#each sections as { section, href: sectionLink } (section.id)}
								{@const active = isActive(page.url.pathname, sectionLink)}
								<Sidebar.MenuSubItem>
									<Sidebar.MenuSubButton
										href={sectionLink}
										isActive={active}
										aria-current={active ? 'page' : undefined}
									>
										<span>{section.title()}</span>
									</Sidebar.MenuSubButton>
								</Sidebar.MenuSubItem>
							{/each}
						</Sidebar.MenuSub>
					</Collapsible.Content>
				</Sidebar.MenuItem>
			{/snippet}
		</Collapsible.Root>
	</Sidebar.Menu>
</Sidebar.Group>
