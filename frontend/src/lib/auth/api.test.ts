import { describe, expect, it } from 'vitest';
import { oidcErrorMessage, oidcLoginHref, ssoButton } from './api';

const capabilities = {
	local_enabled: true,
	registration_enabled: true,
	require_login: false,
	oidc_enabled: true,
	oidc_name: 'Authentik'
};

describe('single sign-on', () => {
	it('starts at the API with the redirect target', () => {
		expect(oidcLoginHref('/')).toBe('/api/v0/auth/oidc/login');
		expect(oidcLoginHref('/watch?v=x')).toBe('/api/v0/auth/oidc/login?redirect=%2Fwatch%3Fv%3Dx');
		expect(oidcLoginHref('https://evil.example/')).toBe('/api/v0/auth/oidc/login');
	});

	it('labels the button with the provider name', () => {
		expect(ssoButton(capabilities, '/feed')).toEqual({
			label: 'Sign in with Authentik',
			href: '/api/v0/auth/oidc/login?redirect=%2Ffeed'
		});
		expect(ssoButton({ ...capabilities, oidc_name: null }, '/')?.label).toBe('Sign in with SSO');
		expect(ssoButton({ ...capabilities, oidc_enabled: false }, '/')).toBeUndefined();
		expect(ssoButton(null, '/')).toBeUndefined();
	});

	it('explains callback errors', () => {
		expect(oidcErrorMessage(null)).toBeUndefined();
		expect(oidcErrorMessage('oidc_denied')).toBe('Sign-in was cancelled at the provider.');
		expect(oidcErrorMessage('account_disabled')).toContain('disabled');
		expect(oidcErrorMessage('something_new')).toBe(
			'Single sign-on did not complete. Start again from this page.'
		);
	});
});
