<script lang="ts">
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { failureMessage, register, ssoButton } from '#lib/auth/api.js';
	import { authHref, redirectTarget } from '#lib/auth/links.js';
	import {
		PASSWORD_MAX_LENGTH,
		PASSWORD_MIN_LENGTH,
		validateRegister,
		type Errors,
		type FieldError,
		type RegisterInput
	} from '#lib/auth/validation.js';
	import AuthForm from '#lib/components/auth/auth-form.svelte';
	import FormField from '#lib/components/auth/form-field.svelte';
	import EmptyState from '#lib/components/state/empty-state.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { session } from '#lib/session.svelte.js';

	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let errors = $state<Errors<RegisterInput>>({});
	let status = $state('');
	let busy = $state(false);

	const target = $derived(redirectTarget(page.url));
	const capabilities = $derived(session.capabilities);
	const closed = $derived(
		capabilities !== null && (!capabilities.local_enabled || !capabilities.registration_enabled)
	);

	$effect(() => {
		if (session.account) void goto(target, { replaceState: true });
	});

	const min = String(PASSWORD_MIN_LENGTH);
	const max = String(PASSWORD_MAX_LENGTH);

	const messages: Record<keyof RegisterInput, Partial<Record<FieldError, string>>> = {
		username: {
			required: m.auth_error_username_required(),
			format: m.auth_error_username_format(),
			taken: m.auth_error_username_taken()
		},
		password: {
			required: m.auth_error_password_required(),
			length: m.auth_error_password_length({ min }),
			too_long: m.auth_error_password_too_long({ max })
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

	async function showErrors(form: HTMLFormElement) {
		const count = Object.keys(errors).length;
		status = m.auth_error_summary({
			count,
			formatted: new Intl.NumberFormat(getLocale()).format(count)
		});
		await tick();
		form.querySelector<HTMLInputElement>('[aria-invalid="true"]')?.focus();
	}

	async function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const form = event.currentTarget as HTMLFormElement;
		errors = validateRegister({ username, password, confirmPassword });
		if (Object.keys(errors).length > 0) return showErrors(form);

		busy = true;
		status = '';
		const result = await register({ username: username.trim(), password });
		busy = false;
		if (result.ok) {
			session.signedIn(result.account);
			return;
		}
		const fields = result.error?.fields;
		if (fields && Object.keys(fields).length > 0) {
			errors = {
				...(fields.username && { username: fields.username }),
				...(fields.password && { password: fields.password })
			};
			return showErrors(form);
		}
		status = failureMessage(result.status, result.error);
	}
</script>

<svelte:head>
	<title>{pageTitle(m.user_register())}</title>
</svelte:head>

{#if closed}
	<div class="w-full max-w-sm" data-testid="registration-closed">
		<EmptyState
			title={m.auth_registration_closed_title()}
			description={m.auth_registration_closed_body()}
		>
			{#snippet actions()}
				<Button href={authHref('/login', target)}>{m.user_sign_in()}</Button>
			{/snippet}
		</EmptyState>
	</div>
{:else}
	<AuthForm
		title={m.auth_register_title()}
		subtitle={m.auth_register_subtitle()}
		submitLabel={busy ? m.auth_registering() : m.auth_register_submit()}
		{status}
		{busy}
		{onsubmit}
		sso={ssoButton(capabilities, target)}
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
				href={authHref('/login', target)}
				class="font-medium text-foreground underline decoration-border underline-offset-4 transition-colors duration-(--motion-in) ease-ui hover:decoration-foreground"
			>
				{m.auth_login_link()}
			</a>
		{/snippet}
	</AuthForm>
{/if}
