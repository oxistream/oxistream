import { describe, expect, it } from 'vitest';
import { redirectQuery, safeRedirect } from './redirect';

describe('safeRedirect', () => {
	it('keeps same-origin paths', () => {
		expect(safeRedirect('/feed/history')).toBe('/feed/history');
		expect(safeRedirect('/watch?v=dQw4w9WgXcQ#t=10')).toBe('/watch?v=dQw4w9WgXcQ#t=10');
	});

	it('rejects other origins and auth pages', () => {
		for (const target of [
			null,
			'',
			'https://evil.example/',
			'//evil.example',
			'/\\evil.example',
			'javascript:alert(1)',
			'/login?redirect=/',
			'/register'
		]) {
			expect(safeRedirect(target)).toBe('/');
		}
	});
});

describe('redirectQuery', () => {
	it('encodes the target and omits home', () => {
		expect(redirectQuery('/watch?v=x')).toBe('?redirect=%2Fwatch%3Fv%3Dx');
		expect(redirectQuery('/')).toBe('');
	});
});
