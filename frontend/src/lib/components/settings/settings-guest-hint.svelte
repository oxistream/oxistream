<script lang="ts">
	import { resolve } from '$app/paths';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
	import { cn } from '#lib/utils.js';

	let { class: className }: { class?: string } = $props();

	const link = 'font-medium text-primary underline-offset-4 hover:underline';
</script>

<p
	data-testid="settings-guest-hint"
	class={cn('flex flex-wrap items-baseline gap-x-3 gap-y-1 text-sm', className)}
>
	<span class="text-muted-foreground">{m.settings_guest_title()}</span>
	<span class="flex gap-3">
		<a href={resolve('/login')} class={link}>{m.user_sign_in()}</a>
		{#if session.capabilities?.registration_enabled !== false}
			<a href={resolve('/register')} class={link}>{m.user_register()}</a>
		{/if}
	</span>
</p>
