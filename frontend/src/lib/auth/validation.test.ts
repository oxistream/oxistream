import { describe, expect, it } from 'vitest';
import {
	PASSWORD_MAX_LENGTH,
	PASSWORD_MIN_LENGTH,
	validateLogin,
	validateRegister
} from './validation';

const strong = 'x'.repeat(PASSWORD_MIN_LENGTH);

describe('validateLogin', () => {
	it('requires both fields', () => {
		expect(validateLogin({ username: ' ', password: '' })).toEqual({
			username: 'required',
			password: 'required'
		});
	});

	it('accepts any filled credentials', () => {
		expect(validateLogin({ username: 'mara', password: 'short' })).toEqual({});
	});
});

describe('validateRegister', () => {
	it('checks the username format', () => {
		expect(
			validateRegister({ username: 'a b', password: strong, confirmPassword: strong }).username
		).toBe('format');
		expect(
			validateRegister({ username: 'mara.l-2', password: strong, confirmPassword: strong })
		).toEqual({});
	});

	it('enforces the minimum password length', () => {
		const short = 'x'.repeat(PASSWORD_MIN_LENGTH - 1);
		expect(
			validateRegister({ username: 'mara', password: short, confirmPassword: short }).password
		).toBe('length');
	});

	it('enforces the maximum password length in code points', () => {
		const long = 'x'.repeat(PASSWORD_MAX_LENGTH + 1);
		expect(
			validateRegister({ username: 'mara', password: long, confirmPassword: long }).password
		).toBe('too_long');
		const emoji = '🦀'.repeat(PASSWORD_MAX_LENGTH);
		expect(validateRegister({ username: 'mara', password: emoji, confirmPassword: emoji })).toEqual(
			{}
		);
	});

	it('requires matching passwords', () => {
		expect(
			validateRegister({ username: 'mara', password: strong, confirmPassword: `${strong}!` })
				.confirmPassword
		).toBe('mismatch');
	});
});
