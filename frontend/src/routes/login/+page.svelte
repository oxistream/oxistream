<script lang="ts">
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { failureMessage, login, oidcErrorMessage, ssoButton } from '#lib/auth/api.js';
	import { authHref, redirectTarget } from '#lib/auth/links.js';
	import { validateLogin, type Errors, type LoginInput } from '#lib/auth/validation.js';
	import AuthForm from '#lib/components/auth/auth-form.svelte';
	import FormField from '#lib/components/auth/form-field.svelte';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	let username = $state('');
	let password = $state('');
	let errors = $state<Errors<LoginInput>>({});
	// A failed single sign-on lands here with `?error=`.
	let status = $state(oidcErrorMessage(page.url.searchParams.get('error')) ?? '');
	let busy = $state(false);

	const target = $derived(redirectTarget(page.url));
	const capabilities = $derived(session.capabilities);
	const sso = $derived(ssoButton(capabilities, target));

	$effect(() => {
		// Signed in already, or just now: continue where the user came from.
		if (session.account) void goto(target, { replaceState: true });
	});

	const messages = {
		username: { required: m.auth_error_username_required() },
		password: { required: m.auth_error_password_required() }
	} as const;

	function errorFor(field: keyof LoginInput) {
		const code = errors[field];
		return code === 'required' ? messages[field].required : undefined;
	}

	async function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const form = event.currentTarget as HTMLFormElement;
		errors = validateLogin({ username, password });
		const count = Object.keys(errors).length;
		if (count > 0) {
			status = m.auth_error_summary({
				count,
				formatted: new Intl.NumberFormat(getLocale()).format(count)
			});
			await tick();
			form.querySelector<HTMLInputElement>('[aria-invalid="true"]')?.focus();
			return;
		}

		busy = true;
		status = '';
		const result = await login({ username: username.trim(), password });
		busy = false;
		if (result.ok) {
			session.signedIn(result.account);
			return;
		}
		password = '';
		status = failureMessage(result.status, result.error);
		await tick();
		form.querySelector<HTMLInputElement>('#password')?.focus();
	}
</script>

<svelte:head>
	<title>{pageTitle(m.user_sign_in())}</title>
</svelte:head>

<AuthForm
	title={m.auth_login_title()}
	subtitle={m.auth_login_subtitle()}
	submitLabel={busy ? m.auth_signing_in() : m.auth_sign_in_submit()}
	local={capabilities?.local_enabled !== false}
	{status}
	{busy}
	{onsubmit}
	{sso}
>
	<FormField
		id="username"
		name="username"
		label={m.auth_username()}
		autocomplete="username"
		bind:value={username}
		error={errorFor('username')}
	/>
	<FormField
		id="password"
		name="password"
		type="password"
		label={m.auth_password()}
		autocomplete="current-password"
		bind:value={password}
		error={errorFor('password')}
	/>
	{#snippet footer()}
		{#if capabilities?.registration_enabled !== false}
			{m.auth_no_account()}
			<a
				href={authHref('/register', target)}
				class="font-medium text-foreground underline decoration-border underline-offset-4 transition-colors duration-(--motion-in) ease-ui hover:decoration-foreground"
			>
				{m.auth_register_link()}
			</a>
		{/if}
	{/snippet}
</AuthForm>
