// Mirrors the backend policy in `oxistream-server/src/auth/password.rs`.
export const PASSWORD_MIN_LENGTH = 12;
export const PASSWORD_MAX_LENGTH = 256;

const USERNAME_PATTERN = /^[A-Za-z0-9._-]{3,32}$/;

export type FieldError = 'required' | 'format' | 'length' | 'too_long' | 'taken' | 'mismatch';

export type LoginInput = { username: string; password: string };
export type RegisterInput = LoginInput & { confirmPassword: string };

export type Errors<T> = Partial<Record<keyof T, FieldError>>;

/** Counted in code points, like the backend. */
function length(value: string): number {
	return [...value].length;
}

export function validateLogin(input: LoginInput): Errors<LoginInput> {
	const errors: Errors<LoginInput> = {};
	if (!input.username.trim()) errors.username = 'required';
	if (!input.password) errors.password = 'required';
	return errors;
}

export function validateRegister(input: RegisterInput): Errors<RegisterInput> {
	const errors: Errors<RegisterInput> = {};
	const username = input.username.trim();
	if (!username) errors.username = 'required';
	else if (!USERNAME_PATTERN.test(username)) errors.username = 'format';

	if (!input.password) errors.password = 'required';
	else if (length(input.password) < PASSWORD_MIN_LENGTH) errors.password = 'length';
	else if (length(input.password) > PASSWORD_MAX_LENGTH) errors.password = 'too_long';

	if (!input.confirmPassword) errors.confirmPassword = 'required';
	else if (input.confirmPassword !== input.password) errors.confirmPassword = 'mismatch';
	return errors;
}
