<script lang="ts">
	import { resolve } from '$app/paths';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { session } from '#lib/session.svelte.js';
</script>

<Button href={resolve('/login')}>{m.user_sign_in()}</Button>
{#if session.capabilities?.registration_enabled !== false}
	<Button href={resolve('/register')} variant="outline">{m.user_register()}</Button>
{/if}
