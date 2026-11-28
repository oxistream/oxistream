<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { m } from '#lib/paraglide/messages.js';
	import AuthForm from './auth-form.svelte';
	import FormField from './form-field.svelte';

	const { Story } = defineMeta({
		title: 'Auth/AuthForm',
		component: AuthForm
	});

	const noop = (event: SubmitEvent) => event.preventDefault();
	const sso = { label: m.auth_oidc_sign_in({ name: 'Authentik' }), href: '#' };
</script>

<Story name="Sign in" asChild>
	<AuthForm
		title={m.auth_login_title()}
		submitLabel={m.auth_sign_in_submit()}
		onsubmit={noop}
		{sso}
	>
		<FormField id="username" name="username" label={m.auth_username()} />
		<FormField id="password" name="password" type="password" label={m.auth_password()} />
		{#snippet footer()}{m.auth_no_account()}{/snippet}
	</AuthForm>
</Story>

<Story name="Validation errors" asChild>
	<AuthForm
		title={m.auth_register_title()}
		submitLabel={m.auth_register_submit()}
		status={m.auth_error_summary({ count: 2, formatted: '2' })}
		onsubmit={noop}
		{sso}
	>
		<FormField
			id="username"
			name="username"
			label={m.auth_username()}
			value="a b"
			error={m.auth_error_username_format()}
		/>
		<FormField
			id="password"
			name="password"
			type="password"
			label={m.auth_password()}
			value="short"
			error={m.auth_error_password_length({ min: '12' })}
		/>
		<FormField
			id="confirm"
			name="confirm"
			type="password"
			label={m.auth_password_confirm()}
			hint={m.auth_password_hint({ min: '12' })}
		/>
		{#snippet footer()}{m.auth_have_account()}{/snippet}
	</AuthForm>
</Story>

<Story name="Single sign-on only" asChild>
	<AuthForm
		title={m.auth_login_title()}
		submitLabel={m.auth_sign_in_submit()}
		local={false}
		onsubmit={noop}
		{sso}
	>
		<span></span>
	</AuthForm>
</Story>
