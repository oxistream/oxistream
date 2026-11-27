<script lang="ts">
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import type { Snippet } from 'svelte';
	import type { AccountResult } from '#lib/api/settings.js';
	import FormField from '#lib/components/auth/form-field.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button, buttonVariants } from '#lib/components/ui/button/index.js';
	import { PASSWORD_MAX_LENGTH, PASSWORD_MIN_LENGTH } from '#lib/auth/validation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import type { Account } from '#lib/session.svelte.js';
	import SettingRow from './setting-row.svelte';

	let {
		account,
		localEnabled = true,
		onchangepassword,
		ondelete,
		children
	}: {
		account: Account;
		/** Password changes need local accounts to be enabled on the instance. */
		localEnabled?: boolean;
		onchangepassword: (body: {
			current_password: string;
			new_password: string;
		}) => Promise<AccountResult>;
		/** Resolves once the account is gone or the attempt failed. */
		ondelete: (body: { username: string; password?: string }) => Promise<AccountResult>;
		/** Rows shown below the account summary. */
		children?: Snippet;
	} = $props();

	const since = $derived(
		new Intl.DateTimeFormat(getLocale(), { dateStyle: 'long' }).format(new Date(account.created_at))
	);

	let current = $state('');
	let next = $state('');
	let confirm = $state('');
	let passwordErrors = $state<{ current?: string; next?: string; confirm?: string }>({});
	let passwordStatus = $state('');
	let passwordBusy = $state(false);

	function generic(result: Extract<AccountResult, { ok: false }>): string {
		if (result.status === 429) return m.auth_error_too_many_attempts();
		if (result.error?.code === 'local_auth_disabled') return m.auth_local_disabled();
		return m.auth_error_generic();
	}

	async function submitPassword(event: SubmitEvent) {
		event.preventDefault();
		const errors: typeof passwordErrors = {};
		const length = [...next].length;
		if (!current) errors.current = m.auth_error_password_required();
		if (length < PASSWORD_MIN_LENGTH)
			errors.next = m.auth_error_password_length({ min: PASSWORD_MIN_LENGTH });
		else if (length > PASSWORD_MAX_LENGTH)
			errors.next = m.auth_error_password_too_long({ max: PASSWORD_MAX_LENGTH });
		if (confirm !== next) errors.confirm = m.auth_error_password_mismatch();
		passwordErrors = errors;
		passwordStatus = '';
		if (Object.keys(errors).length > 0) return;

		passwordBusy = true;
		const result = await onchangepassword({ current_password: current, new_password: next });
		passwordBusy = false;
		if (result.ok) {
			current = next = confirm = '';
			passwordStatus = m.settings_password_changed();
			return;
		}
		const fields = result.error?.fields ?? {};
		if (fields.current_password === 'mismatch') {
			passwordErrors = { current: m.settings_password_wrong() };
		} else if (fields.new_password) {
			passwordErrors = { next: m.auth_error_password_length({ min: PASSWORD_MIN_LENGTH }) };
		} else {
			passwordStatus = generic(result);
		}
	}

	let deleteOpen = $state(false);
	let confirmName = $state('');
	let deletePassword = $state('');
	let deleteError = $state('');
	let deleteBusy = $state(false);
	const nameMatches = $derived(confirmName.trim().toLowerCase() === account.username.toLowerCase());

	$effect(() => {
		if (!deleteOpen) {
			confirmName = deletePassword = deleteError = '';
		}
	});

	async function submitDelete(event: SubmitEvent) {
		event.preventDefault();
		if (!nameMatches || (account.has_password && !deletePassword)) return;
		deleteBusy = true;
		deleteError = '';
		const result = await ondelete({
			username: confirmName.trim(),
			password: account.has_password ? deletePassword : undefined
		});
		deleteBusy = false;
		if (result.ok) {
			deleteOpen = false;
			return;
		}
		const fields = result.error?.fields ?? {};
		if (result.error?.code === 'last_admin') deleteError = m.settings_delete_last_admin();
		else if (fields.password === 'mismatch') deleteError = m.settings_password_wrong();
		else if (fields.username === 'mismatch') deleteError = m.settings_delete_name_mismatch();
		else deleteError = generic(result);
	}
</script>

<SettingRow
	label={account.username}
	description={account.has_password
		? m.settings_account_local({ date: since })
		: m.settings_account_sso({ date: since })}
>
	{#snippet control()}
		<span
			class="inline-flex h-6 items-center rounded-md bg-muted px-2 text-xs font-medium text-muted-foreground"
		>
			{account.role === 'admin' ? m.settings_role_admin() : m.settings_role_user()}
		</span>
	{/snippet}
</SettingRow>

{@render children?.()}

{#if account.has_password && localEnabled}
	<SettingRow label={m.settings_password()} description={m.settings_password_hint()} stacked>
		<form
			class="flex max-w-sm flex-col gap-4"
			novalidate
			onsubmit={submitPassword}
			aria-busy={passwordBusy}
		>
			<!-- Lets password managers pair the new password with the account. -->
			<input type="text" name="username" value={account.username} autocomplete="username" hidden />
			<FormField
				id="settings-current-password"
				name="current-password"
				type="password"
				autocomplete="current-password"
				label={m.settings_password_current()}
				bind:value={current}
				error={passwordErrors.current}
			/>
			<FormField
				id="settings-new-password"
				name="new-password"
				type="password"
				autocomplete="new-password"
				label={m.settings_password_new()}
				hint={m.auth_password_hint({ min: PASSWORD_MIN_LENGTH })}
				bind:value={next}
				error={passwordErrors.next}
			/>
			<FormField
				id="settings-confirm-password"
				name="confirm-password"
				type="password"
				autocomplete="new-password"
				label={m.auth_password_confirm()}
				bind:value={confirm}
				error={passwordErrors.confirm}
			/>
			<div class="flex flex-wrap items-center gap-3">
				<Button type="submit" disabled={passwordBusy}>{m.settings_password_submit()}</Button>
				<p role="status" class="text-sm text-muted-foreground" data-testid="password-status">
					{passwordStatus}
				</p>
			</div>
		</form>
	</SettingRow>
{/if}

<SettingRow label={m.settings_delete()} description={m.settings_delete_hint()}>
	{#snippet control()}
		<AlertDialog.Root bind:open={deleteOpen}>
			<AlertDialog.Trigger class={buttonVariants({ variant: 'destructive', size: 'sm' })}>
				<Trash2Icon />
				{m.settings_delete()}
			</AlertDialog.Trigger>
			<AlertDialog.Content>
				<form class="flex flex-col gap-4" novalidate onsubmit={submitDelete}>
					<AlertDialog.Header>
						<AlertDialog.Title>{m.settings_delete_title()}</AlertDialog.Title>
						<AlertDialog.Description>{m.settings_delete_body()}</AlertDialog.Description>
					</AlertDialog.Header>
					<FormField
						id="settings-delete-username"
						name="confirm-username"
						label={m.settings_delete_type_name({ username: account.username })}
						autocomplete="off"
						bind:value={confirmName}
					/>
					{#if account.has_password}
						<FormField
							id="settings-delete-password"
							name="password"
							type="password"
							autocomplete="current-password"
							label={m.auth_password()}
							bind:value={deletePassword}
						/>
					{/if}
					{#if deleteError}
						<p role="alert" class="text-sm font-medium text-destructive">{deleteError}</p>
					{/if}
					<AlertDialog.Footer>
						<AlertDialog.Cancel type="button">{m.action_cancel()}</AlertDialog.Cancel>
						<Button
							type="submit"
							variant="destructive"
							disabled={deleteBusy || !nameMatches || (account.has_password && !deletePassword)}
						>
							{m.settings_delete_confirm()}
						</Button>
					</AlertDialog.Footer>
				</form>
			</AlertDialog.Content>
		</AlertDialog.Root>
	{/snippet}
</SettingRow>
