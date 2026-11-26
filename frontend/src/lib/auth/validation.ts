export const PASSWORD_MIN_LENGTH = 12;

const USERNAME_PATTERN = /^[A-Za-z0-9._-]{3,32}$/;

export type FieldError = 'required' | 'format' | 'length' | 'mismatch';

export type LoginInput = { username: string; password: string };
export type RegisterInput = LoginInput & { confirmPassword: string };

export type Errors<T> = Partial<Record<keyof T, FieldError>>;

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
	else if (input.password.length < PASSWORD_MIN_LENGTH) errors.password = 'length';

	if (!input.confirmPassword) errors.confirmPassword = 'required';
	else if (input.confirmPassword !== input.password) errors.confirmPassword = 'mismatch';
	return errors;
}
