<script lang="ts">
	import { resolve } from '$app/paths';
	import { Button } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';
</script>

<Button href={resolve('/login')}>{m.user_sign_in()}</Button>
<Button href={resolve('/register')} variant="outline">{m.user_register()}</Button>
