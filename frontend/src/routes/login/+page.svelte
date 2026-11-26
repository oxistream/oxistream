<script lang="ts">
	import { tick } from 'svelte';
	import { resolve } from '$app/paths';
	import AuthForm from '#lib/components/auth/auth-form.svelte';
	import FormField from '#lib/components/auth/form-field.svelte';
	import { validateLogin, type Errors, type LoginInput } from '#lib/auth/validation.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';

	let username = $state('');
	let password = $state('');
	let errors = $state<Errors<LoginInput>>({});
	let status = $state('');

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
		// No-op until local auth lands (#8).
		status = m.auth_not_connected();
	}

	function onsso() {
		// No-op until OIDC lands (#9).
		status = m.auth_not_connected();
	}
</script>

<svelte:head>
	<title>{pageTitle(m.user_sign_in())}</title>
</svelte:head>

<AuthForm
	title={m.auth_login_title()}
	subtitle={m.auth_login_subtitle()}
	submitLabel={m.auth_sign_in_submit()}
	{status}
	{onsubmit}
	{onsso}
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
		{m.auth_no_account()}
		<a
			href={resolve('/register')}
			class="font-medium text-foreground underline decoration-border underline-offset-4 transition-colors duration-(--motion-in) ease-ui hover:decoration-foreground"
		>
			{m.auth_register_link()}
		</a>
	{/snippet}
</AuthForm>
