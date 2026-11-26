<script lang="ts">
	import { tick } from 'svelte';
	import { resolve } from '$app/paths';
	import AuthForm from '#lib/components/auth/auth-form.svelte';
	import FormField from '#lib/components/auth/form-field.svelte';
	import {
		PASSWORD_MIN_LENGTH,
		validateRegister,
		type Errors,
		type FieldError,
		type RegisterInput
	} from '#lib/auth/validation.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';

	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let errors = $state<Errors<RegisterInput>>({});
	let status = $state('');

	const min = String(PASSWORD_MIN_LENGTH);

	const messages: Record<keyof RegisterInput, Partial<Record<FieldError, string>>> = {
		username: {
			required: m.auth_error_username_required(),
			format: m.auth_error_username_format()
		},
		password: {
			required: m.auth_error_password_required(),
			length: m.auth_error_password_length({ min })
		},
		confirmPassword: {
			required: m.auth_error_password_required(),
			mismatch: m.auth_error_password_mismatch()
		}
	};

	function errorFor(field: keyof RegisterInput) {
		const code = errors[field];
		return code ? messages[field][code] : undefined;
	}

	async function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		const form = event.currentTarget as HTMLFormElement;
		errors = validateRegister({ username, password, confirmPassword });
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
	<title>{pageTitle(m.user_register())}</title>
</svelte:head>

<AuthForm
	title={m.auth_register_title()}
	subtitle={m.auth_register_subtitle()}
	submitLabel={m.auth_register_submit()}
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
		autocomplete="new-password"
		hint={m.auth_password_hint({ min })}
		bind:value={password}
		error={errorFor('password')}
	/>
	<FormField
		id="confirm-password"
		name="confirm-password"
		type="password"
		label={m.auth_password_confirm()}
		autocomplete="new-password"
		bind:value={confirmPassword}
		error={errorFor('confirmPassword')}
	/>
	{#snippet footer()}
		{m.auth_have_account()}
		<a
			href={resolve('/login')}
			class="font-medium text-foreground underline decoration-border underline-offset-4 transition-colors duration-(--motion-in) ease-ui hover:decoration-foreground"
		>
			{m.auth_login_link()}
		</a>
	{/snippet}
</AuthForm>
